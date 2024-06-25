use std::{
    sync::atomic::{AtomicU64, Ordering},
    time::Duration,
};

use reqwest::{Client, IntoUrl, Url};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::Value;

use crate::HTTP_USER_AGENT;

#[derive(Deserialize, Serialize)]
pub struct JSONRPCRequest {
    pub jsonrpc: String,
    pub id: u64,
    pub method: String,
    pub params: Vec<Value>,
}

impl JSONRPCRequest {
    pub fn new<'a, M, I>(id: u64, method: M, params: I) -> JSONRPCRequest
    where
        M: Into<String>,
        I: IntoIterator<Item = &'a Value>,
    {
        JSONRPCRequest {
            jsonrpc: "2.0".to_string(),
            id,
            method: method.into(),
            params: params.into_iter().cloned().collect(),
        }
    }
}

#[derive(Deserialize, Serialize)]
pub struct JSONRPCResponse<T> {
    pub jsonrpc: String,
    pub id: u64,
    pub result: T,
}

pub struct SolanaRPCClient {
    client: Client,
    endpoint: Url,
    counter: AtomicU64,
}

impl SolanaRPCClient {
    pub fn new<U: IntoUrl>(endpoint: U, timeout: Duration) -> SolanaRPCClient {
        SolanaRPCClient {
            client: Client::builder()
                .user_agent(HTTP_USER_AGENT)
                .timeout(timeout)
                .build()
                .unwrap(),
            endpoint: endpoint.into_url().unwrap(),
            counter: AtomicU64::new(1),
        }
    }

    pub async fn send<'a, M, I, R>(&self, method: M, params: I) -> Result<R, reqwest::Error>
    where
        M: Into<String>,
        I: IntoIterator<Item = &'a Value>,
        R: DeserializeOwned,
    {
        let id = self.counter.fetch_add(1, Ordering::SeqCst);
        let payload = JSONRPCRequest::new(id, method, params);

        let response: JSONRPCResponse<R> = self
            .client
            .post(self.endpoint.clone())
            .header("content-type", "application/json")
            .json(&payload)
            .send()
            .await?
            .json()
            .await?;

        Ok(response.result)
    }

    pub async fn get_slot(
        &self,
        commitment_config: Option<CommitmentConfig>,
    ) -> Result<u64, reqwest::Error> {
        let commitment_config = serde_json::to_value(commitment_config).unwrap();
        self.send("getSlot", &[commitment_config]).await
    }
}

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
