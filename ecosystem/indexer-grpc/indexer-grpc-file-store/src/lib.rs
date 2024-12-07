// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub mod data_manager;
pub mod file_store_uploader;
pub mod metadata_manager;
pub mod metrics;
pub mod service;

use crate::{
    data_manager::DataManager, metadata_manager::MetadataManager, service::GrpcManagerService,
};
use anyhow::Result;
use aptos_indexer_grpc_server_framework::RunnableConfig;
use aptos_indexer_grpc_utils::{
    config::IndexerGrpcFileStoreConfig,
    status_page::{render_status_page, Tab},
};
use aptos_protos::indexer::v1::{
    grpc_manager_server::GrpcManagerServer, DataServiceInfo, FullnodeInfo,
};
use build_html::{
    Container, ContainerType, HtmlContainer, HtmlElement, HtmlTag, Table, TableCell, TableCellType,
    TableRow,
};
use file_store_uploader::FileStoreUploader;
use futures::executor::block_on;
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, VecDeque},
    net::SocketAddr,
    sync::Arc,
    time::Duration,
};
use tokio::{
    runtime::Handle,
    sync::{Mutex, OnceCell},
};
use tonic::{codec::CompressionEncoding, transport::Server};
use tracing::info;
use warp::{reply::Response, Rejection};

const HTTP2_PING_INTERVAL_DURATION: Duration = Duration::from_secs(60);
const HTTP2_PING_TIMEOUT_DURATION: Duration = Duration::from_secs(10);

static GRPC_MANAGER: OnceCell<GrpcManager> = OnceCell::const_new();

#[derive(Clone, Debug, Deserialize, Serialize)]
struct ServiceConfig {
    listen_address: SocketAddr,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct IndexerGrpcManagerConfig {
    chain_id: u64,
    service_config: ServiceConfig,
    file_store_config: IndexerGrpcFileStoreConfig,
    self_advertised_address: String,
    grpc_manager_addresses: Vec<String>,
    fullnode_addresses: Vec<String>,
}

#[async_trait::async_trait]
impl RunnableConfig for IndexerGrpcManagerConfig {
    async fn run(&self) -> Result<()> {
        GRPC_MANAGER
            .get_or_init(|| async { GrpcManager::new(self).await })
            .await
            .start(&self.service_config);

        Ok(())
    }

    fn get_server_name(&self) -> String {
        "grpc_manager".to_string()
    }

    async fn status_page(&self) -> Result<Response, Rejection> {
        let mut tabs = vec![];

        if let Some(grpc_manager) = GRPC_MANAGER.get() {
            let metadata_manager = grpc_manager.get_metadata_manager();
            tabs.push(render_fullnode_tab(metadata_manager.get_fullnodes_info()));
            let live_data_services_info = metadata_manager.get_live_data_services_info();
            let historical_data_services_info =
                metadata_manager.get_historical_data_services_info();
            tabs.push(render_data_service_tab(
                "LiveDataServices",
                &live_data_services_info,
            ));
            tabs.push(render_data_service_tab(
                "HistoricalDataServices",
                &historical_data_services_info,
            ));
            tabs.push(render_stream_tab(
                &live_data_services_info,
                &historical_data_services_info,
            ));
        }

        render_status_page(tabs)
    }
}

struct GrpcManager {
    chain_id: u64,
    filestore_uploader: Mutex<FileStoreUploader>,
    metadata_manager: Arc<MetadataManager>,
    data_manager: Arc<DataManager>,
}

impl GrpcManager {
    pub(crate) async fn new(config: &IndexerGrpcManagerConfig) -> Self {
        let chain_id = config.chain_id;
        let filestore_uploader = Mutex::new(
            FileStoreUploader::new(chain_id, config.file_store_config.clone())
                .await
                .expect(&format!(
                    "Failed to create filestore uploader, config: {:?}.",
                    config.file_store_config
                )),
        );

        info!(
            chain_id = chain_id,
            "FilestoreUploader is created, config: {:?}.", config.file_store_config
        );

        let metadata_manager = Arc::new(MetadataManager::new(
            config.self_advertised_address.clone(),
            config.grpc_manager_addresses.clone(),
            config.fullnode_addresses.clone(),
        ));

        info!(
            self_advertised_address = config.self_advertised_address,
            "MetadataManager is created, grpc_manager_addresses: {:?}, fullnode_addresses: {:?}.",
            config.grpc_manager_addresses,
            config.fullnode_addresses
        );

        let data_manager = Arc::new(
            DataManager::new(
                chain_id,
                config.file_store_config.clone(),
                filestore_uploader.lock().await.version(),
                metadata_manager.clone(),
            )
            .await,
        );

        info!("DataManager is created.");

        Self {
            chain_id,
            filestore_uploader,
            metadata_manager,
            data_manager,
        }
    }

    pub(crate) fn start(&self, service_config: &ServiceConfig) {
        let service = GrpcManagerServer::new(GrpcManagerService::new(
            self.chain_id,
            self.metadata_manager.clone(),
            self.data_manager.clone(),
        ))
        .send_compressed(CompressionEncoding::Zstd)
        .accept_compressed(CompressionEncoding::Zstd);
        let server = Server::builder()
            .http2_keepalive_interval(Some(HTTP2_PING_INTERVAL_DURATION))
            .http2_keepalive_timeout(Some(HTTP2_PING_TIMEOUT_DURATION))
            .add_service(service);

        tokio_scoped::scope(|s| {
            s.spawn(async move {
                self.metadata_manager.start().await.unwrap();
            });
            s.spawn(async move { self.data_manager.start().await });
            s.spawn(async move {
                self.filestore_uploader
                    .lock()
                    .await
                    .start(self.data_manager.clone())
                    .await
                    .unwrap();
            });
            s.spawn(async move {
                info!("Starting GrpcManager at {}.", service_config.listen_address);
                server.serve(service_config.listen_address).await.unwrap();
            });
        });
    }

    fn get_metadata_manager(&self) -> &MetadataManager {
        &self.metadata_manager
    }
}

fn render_fullnode_tab(fullnodes_info: HashMap<String, VecDeque<FullnodeInfo>>) -> Tab {
    let overview = Container::new(ContainerType::Section)
        .with_paragraph_attr(
            "Connected Fullnodes",
            [("style", "font-size: 24px; font-weight: bold;")],
        )
        .with_table(
            fullnodes_info.into_iter().fold(
                Table::new()
                    .with_attributes([("style", "width: 100%; border: 5px solid black;")])
                    .with_thead_attributes([(
                        "style",
                        "background-color: lightcoral; color: white;",
                    )])
                    .with_custom_header_row(
                        TableRow::new()
                            .with_cell(TableCell::new(TableCellType::Header).with_raw("Id"))
                            .with_cell(
                                TableCell::new(TableCellType::Header)
                                    .with_raw("Last Ping/Heartbeat Time"),
                            )
                            .with_cell(
                                TableCell::new(TableCellType::Header)
                                    .with_raw("Known Latest Version"),
                            ),
                    ),
                |table, fullnode_info| {
                    let last_sample = fullnode_info.1.back();
                    let (timestamp, known_latest_version) = if let Some(last_sample) = last_sample {
                        (
                            format!("{:?}", last_sample.timestamp.unwrap()),
                            format!("{}", last_sample.known_latest_version()),
                        )
                    } else {
                        ("No data point.".to_string(), "No data point.".to_string())
                    };
                    table.with_custom_body_row(
                        TableRow::new()
                            .with_cell(
                                TableCell::new(TableCellType::Data).with_raw(fullnode_info.0),
                            )
                            .with_cell(TableCell::new(TableCellType::Data).with_raw(timestamp))
                            .with_cell(
                                TableCell::new(TableCellType::Data).with_raw(known_latest_version),
                            ),
                    )
                },
            ),
        );
    let content = HtmlElement::new(HtmlTag::Div)
        .with_container(overview)
        .into();

    Tab::new("Fullnodes", content)
}

fn render_data_service_tab(
    tab_name: &str,
    data_services_info: &HashMap<String, VecDeque<DataServiceInfo>>,
) -> Tab {
    let overview = Container::new(ContainerType::Section)
        .with_paragraph_attr(
            format!("Connected {tab_name}"),
            [("style", "font-size: 24px; font-weight: bold;")],
        )
        .with_table(
            data_services_info.iter().fold(
                Table::new()
                    .with_attributes([("style", "width: 100%; border: 5px solid black;")])
                    .with_thead_attributes([(
                        "style",
                        "background-color: lightcoral; color: white;",
                    )])
                    .with_custom_header_row(
                        TableRow::new()
                            .with_cell(TableCell::new(TableCellType::Header).with_raw("Id"))
                            .with_cell(
                                TableCell::new(TableCellType::Header)
                                    .with_raw("Last Ping/Heartbeat Time"),
                            )
                            .with_cell(
                                TableCell::new(TableCellType::Header)
                                    .with_raw("Known Latest Version"),
                            )
                            .with_cell(
                                TableCell::new(TableCellType::Header)
                                    .with_raw("# of Connected Streams"),
                            ),
                    ),
                |table, data_service_info| {
                    let last_sample = data_service_info.1.back();
                    let (timestamp, known_latest_version, num_connected_streams) =
                        if let Some(last_sample) = last_sample {
                            (
                                format!("{:?}", last_sample.timestamp.unwrap()),
                                format!("{}", last_sample.known_latest_version()),
                                format!(
                                    "{}",
                                    last_sample
                                        .stream_info
                                        .as_ref()
                                        .map(|stream_info| stream_info.active_streams.len())
                                        .unwrap_or(0)
                                ),
                            )
                        } else {
                            (
                                "No data point.".to_string(),
                                "No data point.".to_string(),
                                "No data point.".to_string(),
                            )
                        };
                    table.with_custom_body_row(
                        TableRow::new()
                            .with_cell(
                                TableCell::new(TableCellType::Data).with_raw(data_service_info.0),
                            )
                            .with_cell(TableCell::new(TableCellType::Data).with_raw(timestamp))
                            .with_cell(
                                TableCell::new(TableCellType::Data).with_raw(known_latest_version),
                            )
                            .with_cell(
                                TableCell::new(TableCellType::Data).with_raw(num_connected_streams),
                            ),
                    )
                },
            ),
        );
    let content = HtmlElement::new(HtmlTag::Div)
        .with_container(overview)
        .into();

    Tab::new(tab_name, content)
}

fn render_stream_table(data_services_info: &HashMap<String, VecDeque<DataServiceInfo>>) -> Table {
    data_services_info.iter().fold(
        Table::new()
            .with_attributes([("style", "width: 100%; border: 5px solid black;")])
            .with_thead_attributes([("style", "background-color: lightcoral; color: white;")])
            .with_custom_header_row(
                TableRow::new()
                    .with_cell(TableCell::new(TableCellType::Header).with_raw("Stream Id"))
                    .with_cell(TableCell::new(TableCellType::Header).with_raw("Timestamp"))
                    .with_cell(TableCell::new(TableCellType::Header).with_raw("Current Version"))
                    .with_cell(TableCell::new(TableCellType::Header).with_raw("End Version"))
                    .with_cell(
                        TableCell::new(TableCellType::Header).with_raw("Data Service Instance"),
                    ),
            ),
        |mut table, data_service_info| {
            if let Some(last_sample) = data_service_info.1.back() {
                let timestamp = format!("{:?}", last_sample.timestamp.unwrap());
                if let Some(stream_info) = last_sample.stream_info.as_ref() {
                    stream_info.active_streams.iter().for_each(|stream| {
                        table.add_custom_body_row(
                            TableRow::new()
                                .with_cell(
                                    TableCell::new(TableCellType::Data).with_raw(stream.id()),
                                )
                                .with_cell(TableCell::new(TableCellType::Data).with_raw(&timestamp))
                                .with_cell(
                                    TableCell::new(TableCellType::Data)
                                        .with_raw(stream.current_version()),
                                )
                                .with_cell(
                                    TableCell::new(TableCellType::Data)
                                        .with_raw(stream.end_version()),
                                )
                                .with_cell(
                                    TableCell::new(TableCellType::Data)
                                        .with_raw(data_service_info.0),
                                ),
                        )
                    });
                }
            }
            table
        },
    )
}

fn render_stream_tab(
    live_data_services_info: &HashMap<String, VecDeque<DataServiceInfo>>,
    historical_data_services_info: &HashMap<String, VecDeque<DataServiceInfo>>,
) -> Tab {
    let overview = Container::new(ContainerType::Section)
        .with_paragraph_attr(
            format!("Connected Streams"),
            [("style", "font-size: 24px; font-weight: bold;")],
        )
        .with_paragraph_attr(
            format!("LiveDataService Streams"),
            [("style", "font-size: 18px; font-weight: bold;")],
        )
        .with_table(render_stream_table(live_data_services_info))
        .with_paragraph_attr(
            format!("HistoricalDataService Streams"),
            [("style", "font-size: 18px; font-weight: bold;")],
        )
        .with_table(render_stream_table(historical_data_services_info));
    let content = HtmlElement::new(HtmlTag::Div)
        .with_container(overview)
        .into();

    Tab::new("Streams", content)
}
