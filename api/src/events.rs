// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{context::Context, page::Page, param::EventKeyParam};

use diem_api_types::{Error, EventKey, LedgerInfo, Response};

use anyhow::Result;
use warp::{Filter, Rejection, Reply};

pub fn routes(context: Context) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    get_events(context)
}

// GET /events/<event_key>
pub fn get_events(
    context: Context,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::path!("events" / EventKeyParam)
        .and(warp::get())
        .and(warp::query::<Page>())
        .and(context.filter())
        .and_then(handle_get_events)
}

async fn handle_get_events(
    event_key: EventKeyParam,
    page: Page,
    context: Context,
) -> Result<impl Reply, Rejection> {
    Ok(Events::new(event_key, context)?.list(page)?)
}

struct Events {
    key: EventKey,
    ledger_info: LedgerInfo,
    context: Context,
}

impl Events {
    fn new(event_key: EventKeyParam, context: Context) -> Result<Self, Error> {
        let ledger_info = context.get_latest_ledger_info()?;
        Ok(Self {
            key: event_key.parse("event key")?,
            ledger_info,
            context,
        })
    }

    pub fn list(self, page: Page) -> Result<impl Reply, Error> {
        let contract_events = self.context.get_events(
            &self.key.into(),
            page.start(0, u64::MAX)?,
            page.limit()?,
            self.ledger_info.version(),
        )?;

        let converter = self.context.move_converter();
        let events = converter.try_into_events(&contract_events)?;
        Response::new(self.ledger_info, &events)
    }
}
