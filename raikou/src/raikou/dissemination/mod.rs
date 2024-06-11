use crate::raikou::types::*;
use std::{collections::HashSet, future::Future};
use tokio::time::Instant;

pub mod fake;

pub trait DisseminationLayer: Send + Sync + 'static {
    // TODO: accept exclude by ref?
    fn prepare_block(&self, exclude: HashSet<BatchHash>) -> impl Future<Output = Payload> + Send;

    fn prefetch_payload_data(&self, payload: Payload) -> impl Future<Output = ()> + Send;

    fn check_stored(&self, batch: &BatchHash) -> impl Future<Output = bool> + Send;

    fn notify_commit(&self, payloads: Vec<Payload>) -> impl Future<Output = ()> + Send;
}
