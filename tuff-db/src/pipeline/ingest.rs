use crate::db::{OpLog, TuffDb};
use crate::models::Abstract;
use anyhow::Context;
use crate::pipeline::traits::{
    AbstractGenerator, ClaimVerifier, FactFetcher, InputSplitter, VerificationResult,
};

pub struct IngestOutcome {
    pub op: OpLog,
    pub status: crate::models::VerificationStatus,
    pub confidence: f32,
    pub evidence_count: usize,
    pub reason: String,
}

pub struct IngestPipeline<S, F, V, G, D>
where
    S: InputSplitter,
    F: FactFetcher,
    V: ClaimVerifier,
    G: AbstractGenerator,
    D: TuffDb,
{
    pub splitter: S,
    pub fetcher: F,
    pub verifier: V,
    pub generator: G,
    pub db: D,
}

impl<S, F, V, G, D> IngestPipeline<S, F, V, G, D>
where
    S: InputSplitter,
    F: FactFetcher,
    V: ClaimVerifier,
    G: AbstractGenerator,
    D: TuffDb,
{
    pub async fn ingest(&self, input: &str) -> anyhow::Result<Vec<IngestOutcome>> {
        let parts = self.splitter.split(input);
        let mut ops = Vec::new();
        for fragment in parts {
            let facts = self.fetcher.fetch(&fragment).await.context("stage=fetch")?;
            let evidence_count = facts.iter().map(|f| f.evidence.len()).sum();
            let VerificationResult {
                status,
                confidence,
                reason,
            } = self.verifier.verify(&fragment, &facts).await.context("stage=verify")?;
            let abstract_ = self
                .generator
                .generate(&fragment, &facts, status)
                .await.context("stage=abstract")?;
            let op = self.db.append_abstract(abstract_).await.context("stage=db_append")?;
            ops.push(IngestOutcome {
                op,
                status,
                confidence,
                evidence_count,
                reason,
            });
        }
        Ok(ops)
    }

    pub async fn select_all(&self) -> anyhow::Result<Vec<Abstract>> {
        self.db.select(crate::db::SelectQuery::default()).await
    }
}
