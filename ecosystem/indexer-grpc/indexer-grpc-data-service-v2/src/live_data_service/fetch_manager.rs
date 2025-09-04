// Copyright (c) Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    connection_manager::ConnectionManager,
    live_data_service::{data_client::DataClient, data_manager::DataManager},
    metrics::TIMER,
};
use futures::future::{BoxFuture, FutureExt, Shared};
use std::{sync::Arc, time::Duration};
use tokio::sync::RwLock;
use tracing::info;

type FetchTask<'a> = Shared<BoxFuture<'a, usize>>;

pub(super) struct FetchManager<'a> {
    data_manager: Arc<RwLock<DataManager>>,
    data_client: Arc<DataClient>,
    pub(super) fetching_latest_data_task: RwLock<Option<FetchTask<'a>>>,
}

impl<'a> FetchManager<'a> {
    pub(super) fn new(
        data_manager: Arc<RwLock<DataManager>>,
        connection_manager: Arc<ConnectionManager>,
    ) -> Self {
        Self {
            data_manager,
            data_client: Arc::new(DataClient::new(connection_manager)),
            fetching_latest_data_task: RwLock::new(None),
        }
    }

    pub(super) async fn fetch_past_data(&self, version: u64) -> usize {
        let _timer = TIMER.with_label_values(&["fetch_past_data"]).start_timer();
        Self::fetch_and_update_cache(self.data_client.clone(), self.data_manager.clone(), version)
            .await
    }

    pub(super) async fn continuously_fetch_latest_data(&'a self) {
        loop {
            let task = self.fetch_latest_data().boxed().shared();
            *self.fetching_latest_data_task.write().await = Some(task.clone());
            let _ = task.await;
        }
    }

    async fn fetch_and_update_cache(
        data_client: Arc<DataClient>,
        data_manager: Arc<RwLock<DataManager>>,
        version: u64,
    ) -> usize {
        let transactions = data_client.fetch_transactions(version).await;
        let len = transactions.len();

        if len > 0 {
            data_manager
                .write()
                .await
                .update_data(version, transactions);
        }

        len
    }

    async fn fetch_latest_data(&'a self) -> usize {
        let version = self.data_manager.read().await.end_version;
        info!("Fetching latest data starting from version {version}.");
        loop {
            let num_transactions = {
                let _timer = TIMER
                    .with_label_values(&["fetch_latest_data"])
                    .start_timer();
                Self::fetch_and_update_cache(
                    self.data_client.clone(),
                    self.data_manager.clone(),
                    version,
                )
                .await
            };
            if num_transactions != 0 {
                info!("Finished fetching latest data, got {num_transactions} num_transactions starting from version {version}.");
                return num_transactions;
            }
            tokio::time::sleep(Duration::from_millis(200)).await;
        }
    }
}
