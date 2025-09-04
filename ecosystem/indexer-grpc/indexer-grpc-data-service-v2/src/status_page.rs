// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    config::{HISTORICAL_DATA_SERVICE, LIVE_DATA_SERVICE},
    connection_manager::ConnectionManager,
};
use velor_indexer_grpc_utils::status_page::{get_throughput_from_samples, render_status_page, Tab};
use build_html::{
    Container, ContainerType, HtmlContainer, HtmlElement, HtmlTag, Table, TableCell, TableCellType,
    TableRow,
};
use std::time::Duration;
use warp::{reply::Response, Rejection};

pub(crate) fn status_page() -> Result<Response, Rejection> {
    let mut tabs = vec![];
    // TODO(grao): Add something real.
    let overview_tab_content = HtmlElement::new(HtmlTag::Div).with_raw("Welcome!").into();
    tabs.push(Tab::new("Overview", overview_tab_content));
    if let Some(live_data_service) = LIVE_DATA_SERVICE.get() {
        let connection_manager_info =
            render_connection_manager_info(live_data_service.get_connection_manager());
        let cache_info = render_cache_info();
        let content = HtmlElement::new(HtmlTag::Div)
            .with_container(connection_manager_info)
            .with_container(cache_info)
            .into();
        tabs.push(Tab::new("LiveDataService", content));
    }

    if let Some(historical_data_service) = HISTORICAL_DATA_SERVICE.get() {
        let connection_manager_info =
            render_connection_manager_info(historical_data_service.get_connection_manager());
        let file_store_info = render_file_store_info();
        let content = HtmlElement::new(HtmlTag::Div)
            .with_container(connection_manager_info)
            .with_container(file_store_info)
            .into();
        tabs.push(Tab::new("HistoricalDataService", content));
    }

    render_status_page(tabs)
}

fn render_connection_manager_info(connection_manager: &ConnectionManager) -> Container {
    let known_latest_version = connection_manager.known_latest_version();
    let active_streams = connection_manager.get_active_streams();
    let active_streams_table = active_streams.into_iter().fold(
        Table::new()
            .with_attributes([("style", "width: 100%; border: 5px solid black;")])
            .with_thead_attributes([("style", "background-color: lightcoral; color: white;")])
            .with_custom_header_row(
                TableRow::new()
                    .with_cell(TableCell::new(TableCellType::Header).with_raw("Id"))
                    .with_cell(TableCell::new(TableCellType::Header).with_raw("Current Version"))
                    .with_cell(TableCell::new(TableCellType::Header).with_raw("End Version"))
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
        |table, active_stream| {
            table.with_custom_body_row(
                TableRow::new()
                    .with_cell(TableCell::new(TableCellType::Data).with_raw(&active_stream.id))
                    .with_cell(TableCell::new(TableCellType::Data).with_raw(format!(
                        "{:?}",
                        active_stream.progress.as_ref().and_then(|progress| {
                            progress.samples.last().map(|sample| sample.version)
                        })
                    )))
                    .with_cell(
                        TableCell::new(TableCellType::Data).with_raw(active_stream.end_version()),
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
        },
    );

    Container::new(ContainerType::Section)
        .with_paragraph_attr("Connection Manager", [(
            "style",
            "font-size: 24px; font-weight: bold;",
        )])
        .with_paragraph(format!("Known latest version: {known_latest_version}."))
        .with_paragraph_attr("Active Streams", [(
            "style",
            "font-size: 16px; font-weight: bold;",
        )])
        .with_table(active_streams_table)
}

fn render_cache_info() -> Container {
    Container::new(ContainerType::Section).with_paragraph_attr("In Memory Cache", [(
        "style",
        "font-size: 24px; font-weight: bold;",
    )])
}

fn render_file_store_info() -> Container {
    Container::new(ContainerType::Section).with_paragraph_attr("File Store", [(
        "style",
        "font-size: 24px; font-weight: bold;",
    )])
}
