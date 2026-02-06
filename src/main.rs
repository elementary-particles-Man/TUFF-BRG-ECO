use async_trait::async_trait;
use dotenv::dotenv;
use std::env;
use transformer_neo::db::TuffEngine;
use transformer_neo::pipeline::{
    ClaimVerifier, DummyAbstractGenerator, DummyFetcher, DummySplitter, DummyVerifier, IngestPipeline,
    LlmVerifier,
};
use transformer_neo::models::VerificationStatus;

enum Verifier {
    Dummy(DummyVerifier),
    Llm(LlmVerifier),
}

#[async_trait]
impl ClaimVerifier for Verifier {
    async fn verify(
        &self,
        fragment: &str,
        facts: &[transformer_neo::models::RequiredFact],
    ) -> anyhow::Result<VerificationStatus> {
        match self {
            Verifier::Dummy(v) => v.verify(fragment, facts).await,
            Verifier::Llm(v) => v.verify(fragment, facts).await,
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv().ok();

    let engine = TuffEngine::new("tuff.wal")?;

    let verifier = match env::var("OPENAI_API_KEY") {
        Ok(key) if !key.trim().is_empty() => {
            let model = env::var("OPENAI_MODEL").unwrap_or_else(|_| "gpt-4o-mini".to_string());
            Verifier::Llm(LlmVerifier::new(&key, &model))
        }
        _ => Verifier::Dummy(DummyVerifier),
    };

    let pipeline = IngestPipeline {
        splitter: DummySplitter,
        fetcher: DummyFetcher,
        verifier,
        generator: DummyAbstractGenerator,
        db: engine,
    };

    let ops = pipeline.ingest("SMOKE scenario").await?;
    if let Some(op) = ops.first() {
        println!("op_id={}", op.op_id);
    }

    let all = pipeline.select_all()?;
    println!("stored={}", all.len());
    Ok(())
}
