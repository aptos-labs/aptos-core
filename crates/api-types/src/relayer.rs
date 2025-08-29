use crate::ExecError;
use async_trait::async_trait;
use std::sync::{Arc, OnceLock};

#[async_trait]
pub trait Relayer: Send + Sync + 'static {
    async fn add_uri(&self, uri: &str, rpc_url: &str) -> Result<(), ExecError>;

    async fn get_last_state(&self, uri: &str) -> Result<Vec<u8>, ExecError>;
}

pub static GLOBAL_RELAYER: OnceLock<Arc<dyn Relayer>> = OnceLock::new();
