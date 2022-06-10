use anyhow::Result;
use async_trait::async_trait;

#[cfg(feature = "graphql")]
pub mod gql;

#[cfg(not(feature = "graphql"))]
pub mod tcp;

#[async_trait]
pub trait Connection {
    fn set(&mut self, key: &str, value: &str) -> Result<u64>;
    fn get(&mut self, key: &str) -> Result<u64>;
    fn subscribe(&mut self, key: &str) -> Result<u64>;
    async fn wait_for_ticket(&self, ticket: u64);
}

pub enum Command {
    Init,
    Get(String, u64),
    Set(String, String, u64),
    Subscrube(String, u64),
}