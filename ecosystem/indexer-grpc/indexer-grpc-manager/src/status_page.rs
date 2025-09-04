// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{config::GRPC_MANAGER, data_manager::DataManager};
use velor_indexer_grpc_utils::status_page::{get_throughput_from_samples, render_status_page, Tab};
use velor_protos::{
    indexer::v1::{FullnodeInfo, HistoricalDataServiceInfo, LiveDataServiceInfo, StreamInfo},
    util::timestamp::Timestamp,
};
use build_html::{
    Container, ContainerType, HtmlContainer, HtmlElement, HtmlTag, Table, TableCell, TableCellType,
    TableRow,
};
use std::{
    collections::{HashMap, VecDeque},
    time::Duration,
};
use warp::{reply::Response, Rejection};

pub(crate) async fn status_page() -> Result<Response, Rejection> {
    let mut tabs = vec![];

    if let Some(grpc_manager) = GRPC_MANAGER.get() {
        let data_manager = grpc_manager.get_data_manager();
        tabs.push(render_overview_tab(data_manager).await);
        let metadata_manager = grpc_manager.get_metadata_manager();
        tabs.push(render_fullnode_tab(metadata_manager.get_fullnodes_info()));
        let live_data_services_info = metadata_manager.get_live_data_services_info();
        let historical_data_services_info = metadata_manager.get_historical_data_services_info();
        tabs.push(render_live_data_service_tab(&live_data_services_info));
        tabs.push(render_historical_data_service_tab(
            &historical_data_services_info,
        ));
        tabs.push(render_stream_tab(
            &live_data_services_info,
            &historical_data_services_info,
        ));
    }

    render_status_page(tabs)
}

fn render_fullnode_tab(fullnodes_info: HashMap<String, VecDeque<FullnodeInfo>>) -> Tab {
    let overview = Container::new(ContainerType::Section)
        .with_paragraph_attr("Connected Fullnodes", [(
            "style",
            "font-size: 24px; font-weight: bold;",
        )])
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

fn render_live_data_service_tab(
    data_services_info: &HashMap<String, VecDeque<LiveDataServiceInfo>>,
) -> Tab {
    let column_names = [
        "Id",
        "Last Ping/Heartbeat Time",
        "Known Latest Version",
        "Min Servable Version",
        "# of Connected Streams",
    ];

    let rows = data_services_info
        .iter()
        .map(|entry| {
            let id = entry.0.clone();
            let last_sample = entry.1.back();
            let (timestamp, known_latest_version, min_servable_version, num_connected_streams) =
                if let Some(last_sample) = last_sample {
                    (
                        format!("{:?}", last_sample.timestamp.unwrap()),
                        format!("{}", last_sample.known_latest_version()),
                        format!("{:?}", last_sample.min_servable_version),
                        format!(
                            "{}",
                            last_sample
                                .stream_info
                                .as_ref()
                                .map(|stream_info| stream_info.active_streams.len())
                                .unwrap_or_default()
                        ),
                    )
                } else {
                    (
                        "No data point.".to_string(),
                        "No data point.".to_string(),
                        "No data point.".to_string(),
                        "No data point.".to_string(),
                    )
                };

            [
                id,
                timestamp,
                known_latest_version,
                min_servable_version,
                num_connected_streams,
            ]
        })
        .collect();

    render_data_service_tab("LiveDataServices", column_names, rows)
}

fn render_historical_data_service_tab(
    data_services_info: &HashMap<String, VecDeque<HistoricalDataServiceInfo>>,
) -> Tab {
    let column_names = [
        "Id",
        "Last Ping/Heartbeat Time",
        "Known Latest Version",
        "# of Connected Streams",
    ];

    let rows = data_services_info
        .iter()
        .map(|entry| {
            let id = entry.0.clone();
            let last_sample = entry.1.back();
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
                                .unwrap_or_default()
                        ),
                    )
                } else {
                    (
                        "No data point.".to_string(),
                        "No data point.".to_string(),
                        "No data point.".to_string(),
                    )
                };

            [id, timestamp, known_latest_version, num_connected_streams]
        })
        .collect();

    render_data_service_tab("HistoricalDataServices", column_names, rows)
}

fn render_data_service_tab<const N: usize>(
    tab_name: &str,
    column_names: [&str; N],
    rows: Vec<[String; N]>,
) -> Tab {
    let overview = Container::new(ContainerType::Section)
        .with_paragraph_attr(format!("Connected {tab_name}"), [(
            "style",
            "font-size: 24px; font-weight: bold;",
        )])
        .with_table(
            rows.iter().fold(
                Table::new()
                    .with_attributes([("style", "width: 100%; border: 5px solid black;")])
                    .with_thead_attributes([(
                        "style",
                        "background-color: lightcoral; color: white;",
                    )])
                    .with_custom_header_row(column_names.into_iter().fold(
                        TableRow::new(),
                        |row, column_name| {
                            row.with_cell(
                                TableCell::new(TableCellType::Header).with_raw(column_name),
                            )
                        },
                    )),
                |table, row| {
                    table.with_custom_body_row(row.iter().fold(TableRow::new(), |r, cell| {
                        r.with_cell(TableCell::new(TableCellType::Data).with_raw(cell))
                    }))
                },
            ),
        );
    let content = HtmlElement::new(HtmlTag::Div)
        .with_container(overview)
        .into();

    Tab::new(tab_name, content)
}

fn render_live_data_service_streams(
    data_service_info: &HashMap<String, VecDeque<LiveDataServiceInfo>>,
) -> Table {
    let streams = data_service_info
        .iter()
        .filter_map(|entry| {
            entry.1.back().cloned().and_then(|sample| {
                sample.stream_info.map(|stream_info| {
                    let data_service_instance = entry.0.clone();
                    (
                        data_service_instance,
                        sample.timestamp.unwrap(),
                        stream_info,
                    )
                })
            })
        })
        .collect();

    render_stream_table(streams)
}

fn render_historical_data_service_streams(
    data_service_info: &HashMap<String, VecDeque<HistoricalDataServiceInfo>>,
) -> Table {
    let streams = data_service_info
        .iter()
        .filter_map(|entry| {
            entry.1.back().cloned().and_then(|sample| {
                sample.stream_info.map(|stream_info| {
                    let data_service_instance = entry.0.clone();
                    (
                        data_service_instance,
                        sample.timestamp.unwrap(),
                        stream_info,
                    )
                })
            })
        })
        .collect();

    render_stream_table(streams)
}

fn render_stream_table(streams: Vec<(String, Timestamp, StreamInfo)>) -> Table {
    streams.into_iter().fold(
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
                    )
                    .with_cell(
                        TableCell::new(TableCellType::Header).with_raw("Past 10s throughput"),
                    )
                    .with_cell(
                        TableCell::new(TableCellType::Header).with_raw("Past 60s throughput"),
                    )
                    .with_cell(
                        TableCell::new(TableCellType::Header).with_raw("Past 10min throughput"),
                    ),
            ),
        |mut table, stream| {
            let data_service_instance = stream.0;
            let timestamp = format!("{:?}", stream.1);
            stream.2.active_streams.iter().for_each(|active_stream| {
                table.add_custom_body_row(
                    TableRow::new()
                        .with_cell(TableCell::new(TableCellType::Data).with_raw(&active_stream.id))
                        .with_cell(TableCell::new(TableCellType::Data).with_raw(&timestamp))
                        .with_cell(TableCell::new(TableCellType::Data).with_raw(format!(
                            "{:?}",
                            active_stream.progress.as_ref().and_then(|progress| {
                                progress.samples.last().map(|sample| sample.version)
                            })
                        )))
                        .with_cell(
                            TableCell::new(TableCellType::Data)
                                .with_raw(active_stream.end_version()),
                        )
                        .with_cell(
                            TableCell::new(TableCellType::Data)
                                .with_raw(data_service_instance.as_str()),
                        )
                        .with_cell(TableCell::new(TableCellType::Data).with_raw(
                            get_throughput_from_samples(
                                active_stream.progress.as_ref(),
                                Duration::from_secs(10),
                            ),
                        ))
                        .with_cell(TableCell::new(TableCellType::Data).with_raw(
                            get_throughput_from_samples(
                                active_stream.progress.as_ref(),
                                Duration::from_secs(60),
                            ),
                        ))
                        .with_cell(TableCell::new(TableCellType::Data).with_raw(
                            get_throughput_from_samples(
                                active_stream.progress.as_ref(),
                                Duration::from_secs(600),
                            ),
                        )),
                )
            });
            table
        },
    )
}

fn render_stream_tab(
    live_data_services_info: &HashMap<String, VecDeque<LiveDataServiceInfo>>,
    historical_data_services_info: &HashMap<String, VecDeque<HistoricalDataServiceInfo>>,
) -> Tab {
    let overview = Container::new(ContainerType::Section)
        .with_paragraph_attr("Connected Streams", [(
            "style",
            "font-size: 24px; font-weight: bold;",
        )])
        .with_paragraph_attr("LiveDataService Streams", [(
            "style",
            "font-size: 18px; font-weight: bold;",
        )])
        .with_table(render_live_data_service_streams(live_data_services_info))
        .with_paragraph_attr("HistoricalDataService Streams", [(
            "style",
            "font-size: 18px; font-weight: bold;",
        )])
        .with_table(render_historical_data_service_streams(
            historical_data_services_info,
        ));
    let content = HtmlElement::new(HtmlTag::Div)
        .with_container(overview)
        .into();

    Tab::new("Streams", content)
}

async fn render_overview_tab(data_manager: &DataManager) -> Tab {
    let overview = Container::new(ContainerType::Section)
        .with_paragraph_attr("Cache Stats", [(
            "style",
            "font-size: 24px; font-weight: bold;",
        )])
        .with_paragraph_attr(data_manager.cache_stats().await, [(
            "style",
            "font-size: 16px;",
        )]);

    let content = HtmlElement::new(HtmlTag::Div)
        .with_container(overview)
        .into();

    Tab::new("Overview", content)
}
