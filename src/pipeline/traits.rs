use crate::models::{Abstract, RequiredFact, VerificationStatus};
use async_trait::async_trait;

pub trait InputSplitter: Send + Sync {
    fn split(&self, input: &str) -> Vec<String>;
}

#[async_trait]
pub trait FactFetcher: Send + Sync {
    async fn fetch(&self, fragment: &str) -> anyhow::Result<Vec<RequiredFact>>;
}

#[async_trait]
pub trait ClaimVerifier: Send + Sync {
    async fn verify(
        &self,
        fragment: &str,
        facts: &[RequiredFact],
    ) -> anyhow::Result<VerificationStatus>;
}

#[async_trait]
pub trait AbstractGenerator: Send + Sync {
    async fn generate(
        &self,
        fragment: &str,
        facts: &[RequiredFact],
        status: VerificationStatus,
    ) -> anyhow::Result<Abstract>;
}
