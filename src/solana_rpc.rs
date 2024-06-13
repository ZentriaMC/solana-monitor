use jsonrpsee::proc_macros::rpc;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Commitment {
    // Processed,
    // Confirmed,
    Finalized,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
pub struct CommitmentConfig {
    pub commitment: Commitment,
}

impl CommitmentConfig {
    // pub fn none() -> Option<Self> {
    //     None
    // }

    pub fn commitment(commitment: Commitment) -> Option<Self> {
        Some(Self { commitment })
    }

    // pub fn processed() -> Option<Self> {
    //     Self::commitment(Commitment::Processed)
    // }

    // pub fn confirmed() -> Option<Self> {
    //     Self::commitment(Commitment::Confirmed)
    // }

    pub fn finalized() -> Option<Self> {
        Self::commitment(Commitment::Finalized)
    }
}

#[rpc(client)]
pub trait SolanaRPC {
    #[method(name = "getSlot")]
    fn get_slot(&self, commitment_config: Option<CommitmentConfig>) -> Result<u64, ClientError>;
}
