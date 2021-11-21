// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{
    accounts::Account,
    context::Context,
    failpoint::fail_point,
    metrics::metrics,
    page::Page,
    param::{AddressParam, EventKeyParam, MoveIdentifierParam, MoveStructTagParam},
};

use diem_api_types::{Error, LedgerInfo, Response};

use anyhow::Result;
use diem_types::event::EventKey;
use warp::{filters::BoxedFilter, Filter, Rejection, Reply};

// GET /events/<event_key>
pub fn get_events_by_event_key(context: Context) -> BoxedFilter<(impl Reply,)> {
    warp::path!("events" / EventKeyParam)
        .and(warp::get())
        .and(warp::query::<Page>())
        .and(context.filter())
        .and_then(handle_get_events_by_event_key)
        .with(metrics("get_events_by_event_key"))
        .boxed()
}

// GET /accounts/<address>/events/<event_handle_struct>/<field_name>
pub fn get_events_by_event_handle(context: Context) -> BoxedFilter<(impl Reply,)> {
    warp::path!("accounts" / AddressParam / "events" / MoveStructTagParam / MoveIdentifierParam)
        .and(warp::get())
        .and(warp::query::<Page>())
        .and(context.filter())
        .and_then(handle_get_events_by_event_handle)
        .with(metrics("get_events_by_event_handle"))
        .boxed()
}

async fn handle_get_events_by_event_key(
    event_key: EventKeyParam,
    page: Page,
    context: Context,
) -> Result<impl Reply, Rejection> {
    fail_point("endpoint_get_events_by_event_key")?;
    Ok(Events::new(event_key.parse("event key")?.into(), context)?.list(page)?)
}

async fn handle_get_events_by_event_handle(
    address: AddressParam,
    struct_tag: MoveStructTagParam,
    field_name: MoveIdentifierParam,
    page: Page,
    context: Context,
) -> Result<impl Reply, Rejection> {
    fail_point("endpoint_get_events_by_event_handle")?;
    let key =
        Account::new(None, address, context.clone())?.find_event_key(struct_tag, field_name)?;
    Ok(Events::new(key, context)?.list(page)?)
}

struct Events {
    key: EventKey,
    ledger_info: LedgerInfo,
    context: Context,
}

impl Events {
    fn new(key: EventKey, context: Context) -> Result<Self, Error> {
        let ledger_info = context.get_latest_ledger_info()?;
        Ok(Self {
            key,
            ledger_info,
            context,
        })
    }

    pub fn list(self, page: Page) -> Result<impl Reply, Error> {
        let contract_events = self.context.get_events(
            &self.key,
            page.start(0, u64::MAX)?,
            page.limit()?,
            self.ledger_info.version(),
        )?;

        let converter = self.context.move_converter();
        let events = converter.try_into_events(&contract_events)?;
        Response::new(self.ledger_info, &events)
    }
}
