// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::timestamp_to_unixtime;
use aptos_protos::indexer::v1::StreamProgress;
use build_html::{Html, HtmlChild, HtmlContainer, HtmlElement, HtmlPage, HtmlTag};
use std::time::{Duration, SystemTime};
use warp::{
    reply::{html, Reply, Response},
    Rejection,
};

include!("html.rs");

pub struct Tab {
    name: String,
    content: HtmlChild,
}

impl Tab {
    pub fn new(name: &str, content: HtmlChild) -> Self {
        Self {
            name: name.to_string(),
            content,
        }
    }
}

pub fn render_status_page(tabs: Vec<Tab>) -> Result<Response, Rejection> {
    let tab_names = tabs.iter().map(|tab| tab.name.clone()).collect::<Vec<_>>();
    let tab_contents = tabs.into_iter().map(|tab| tab.content).collect::<Vec<_>>();

    let nav_bar = HtmlElement::new(HtmlTag::Div)
        .with_attribute("id", "nav-bar")
        .with_child(
            tab_names
                .into_iter()
                .enumerate()
                .fold(
                    HtmlElement::new(HtmlTag::UnorderedList),
                    |ul, (i, tab_name)| {
                        ul.with_child(
                            HtmlElement::new(HtmlTag::ListElement)
                                .with_attribute("onclick", format!("showTab({i})"))
                                .with_attribute("class", if i == 0 { "tab active" } else { "tab" })
                                .with_child(tab_name.into())
                                .into(),
                        )
                    },
                )
                .into(),
        );

    let content = tab_contents.into_iter().enumerate().fold(
        HtmlElement::new(HtmlTag::Div),
        |div, (i, tab_content)| {
            div.with_child(
                HtmlElement::new(HtmlTag::Div)
                    .with_attribute("id", format!("tab-{i}"))
                    .with_attribute(
                        "style",
                        if i == 0 {
                            "display: block;"
                        } else {
                            "display: none;"
                        },
                    )
                    .with_child(tab_content)
                    .into(),
            )
        },
    );

    let page = HtmlPage::new()
        .with_title("Status")
        .with_style(STYLE)
        .with_script_literal(SCRIPT)
        .with_raw(nav_bar)
        .with_raw(content)
        .to_html_string();

    Ok(html(page).into_response())
}

pub fn get_throughput_from_samples(
    progress: Option<&StreamProgress>,
    duration: Duration,
) -> String {
    if let Some(progress) = progress {
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs_f64();
        let index = progress.samples.partition_point(|p| {
            let diff = now - timestamp_to_unixtime(p.timestamp.as_ref().unwrap());
            diff > duration.as_secs_f64()
        });

        // Need 2 sample points for calculation.
        // TODO(grao): Consider doing interpolation here.
        if index + 1 < progress.samples.len() {
            let sample_a = &progress.samples[index];
            let sample_b = progress.samples.last().unwrap();
            let time_diff = timestamp_to_unixtime(sample_b.timestamp.as_ref().unwrap())
                - timestamp_to_unixtime(sample_a.timestamp.as_ref().unwrap());
            let tps = (sample_b.version - sample_a.version) as f64 / time_diff;
            let bps = (sample_b.size_bytes - sample_a.size_bytes) as f64 / time_diff;
            return format!(
                "{} tps, {} / s",
                tps as u64,
                bytesize::to_string(bps as u64, /*si_prefix=*/ false)
            );
        }
    }

    "No data".to_string()
}
