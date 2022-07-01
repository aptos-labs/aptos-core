// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    accept_type::AcceptType,
    accounts::Account,
    context::Context,
    failpoint::fail_point,
    metrics::metrics,
    page::Page,
    param::{AddressParam, EventKeyParam, MoveIdentifierParam, MoveStructTagParam},
};

use aptos_api_types::{mime_types::BCS, AsConverter, Error, LedgerInfo, Response};

use anyhow::Result;
use aptos_types::event::EventKey;
use warp::{filters::BoxedFilter, http::header::ACCEPT, Filter, Rejection, Reply};

// GET /events/<event_key>
pub fn get_json_events_by_event_key(context: Context) -> BoxedFilter<(impl Reply,)> {
    warp::path!("events" / EventKeyParam)
        .and(warp::get())
        .and(warp::query::<Page>())
        .and(context.filter())
        .map(|event_key: EventKeyParam, page: Page, context: Context| {
            (event_key, page, context, AcceptType::Json)
        })
        .untuple_one()
        .and_then(handle_get_events_by_event_key)
        .with(metrics("get_json_events_by_event_key"))
        .boxed()
}

// GET /events/<event_key>
pub fn get_bcs_events_by_event_key(context: Context) -> BoxedFilter<(impl Reply,)> {
    warp::path!("events" / EventKeyParam)
        .and(warp::get())
        .and(warp::header::exact(ACCEPT.as_str(), BCS))
        .and(warp::query::<Page>())
        .and(context.filter())
        .map(|event_key: EventKeyParam, page: Page, context: Context| {
            (event_key, page, context, AcceptType::Bcs)
        })
        .untuple_one()
        .and_then(handle_get_events_by_event_key)
        .with(metrics("get_bcs_events_by_event_key"))
        .boxed()
}

// GET /accounts/<address>/events/<event_handle_struct>/<field_name>
pub fn get_json_events_by_event_handle(context: Context) -> BoxedFilter<(impl Reply,)> {
    warp::path!("accounts" / AddressParam / "events" / MoveStructTagParam / MoveIdentifierParam)
        .and(warp::get())
        .and(warp::query::<Page>())
        .and(context.filter())
        .map(|address, struct_tag, field_name, page, context| {
            (
                address,
                struct_tag,
                field_name,
                page,
                context,
                AcceptType::Json,
            )
        })
        .untuple_one()
        .and_then(handle_get_events_by_event_handle)
        .with(metrics("get_events_by_event_handle"))
        .boxed()
}

pub fn get_bcs_events_by_event_handle(context: Context) -> BoxedFilter<(impl Reply,)> {
    warp::path!("accounts" / AddressParam / "events" / MoveStructTagParam / MoveIdentifierParam)
        .and(warp::get())
        .and(warp::header::exact(ACCEPT.as_str(), BCS))
        .and(warp::query::<Page>())
        .and(context.filter())
        .map(|address, struct_tag, field_name, page, context| {
            (
                address,
                struct_tag,
                field_name,
                page,
                context,
                AcceptType::Bcs,
            )
        })
        .untuple_one()
        .and_then(handle_get_events_by_event_handle)
        .with(metrics("get_bcs_events_by_event_handle"))
        .boxed()
}

async fn handle_get_events_by_event_key(
    event_key: EventKeyParam,
    page: Page,
    context: Context,
    accept_type: AcceptType,
) -> Result<impl Reply, Rejection> {
    fail_point("endpoint_get_events_by_event_key")?;
    Ok(Events::new(event_key.parse("event key")?.into(), context)?.list(page, accept_type)?)
}

async fn handle_get_events_by_event_handle(
    address: AddressParam,
    struct_tag: MoveStructTagParam,
    field_name: MoveIdentifierParam,
    page: Page,
    context: Context,
    accept_type: AcceptType,
) -> Result<impl Reply, Rejection> {
    fail_point("endpoint_get_events_by_event_handle")?;
    let key =
        Account::new(None, address, context.clone())?.find_event_key(struct_tag, field_name)?;
    Ok(Events::new(key, context)?.list(page, accept_type)?)
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

    pub fn list(self, page: Page, accept_type: AcceptType) -> Result<impl Reply, Error> {
        let contract_events = self.context.get_events(
            &self.key,
            page.start(0, u64::MAX)?,
            page.limit()?,
            self.ledger_info.version(),
        )?;

        let resolver = self.context.move_resolver()?;
        let events = resolver.as_converter().try_into_events(&contract_events)?;

        match accept_type {
            AcceptType::Json => Response::new(self.ledger_info, &events),
            AcceptType::Bcs => Response::new_bcs(self.ledger_info, &events),
        }
    }
}
