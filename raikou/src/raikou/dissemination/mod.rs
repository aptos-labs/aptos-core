use crate::raikou::types::*;
use std::{collections::HashSet, future::Future};
use tokio::time::Instant;

pub mod fake;

pub trait DisseminationLayer: Send + Sync + 'static {
    // TODO: accept exclude by ref?
    fn pull_payload(&self, exclude: HashSet<BatchInfo>) -> impl Future<Output = Payload> + Send;

    fn prefetch_payload_data(&self, payload: Payload) -> impl Future<Output = ()> + Send;

    fn check_stored(&self, batch: &BatchInfo) -> impl Future<Output = bool> + Send;

    fn notify_commit(&self, payloads: Vec<Payload>) -> impl Future<Output = ()> + Send;
}
