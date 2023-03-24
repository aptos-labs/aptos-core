use std::sync::RwLock;

use crate::{
    database::{
        clean_data_for_db, execute_with_better_error, get_chunks, PgDbPool, PgPoolConnection,
    },
    indexer::{
        errors::TransactionProcessingError, processing_result::ProcessingResult,
        transaction_processor::TransactionProcessor,
    },
    models::{
        events::EventModel, token_models::token_utils::TypeInfo, transactions::TransactionModel,
    },
};
use aptos_api_types::Transaction;
use aptos_types::account_address::AccountAddress;
use async_trait::async_trait;
use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};
use diesel::{result::Error, PgConnection};
use econia_db::models::{
    self,
    events::{NewMakerEvent, NewTakerEvent},
    market::{MarketEventType, NewMarketRegistrationEvent, NewRecognizedMarketEvent},
};
use field_count::FieldCount;
use once_cell::sync::Lazy;
use serde::Deserialize;

pub const NAME: &str = "econia_processor";

static ECONIA_ADDRESS: Lazy<String> =
    Lazy::new(|| std::env::var("ECONIA_ADDRESS").expect("ECONIA_ADDRESS not set"));

static EVENT_TYPES: Lazy<Vec<String>> = Lazy::new(|| {
    vec![
        format!("{}::market::TakerEvent", &*ECONIA_ADDRESS),
        format!("{}::market::MakerEvent", &*ECONIA_ADDRESS),
        format!("{}::registry::MarketRegistrationEvent", &*ECONIA_ADDRESS),
        format!("{}::registry::RecognizedMarketEvent", &*ECONIA_ADDRESS),
    ]
});

static CURRENT_BLOCK_TIME: Lazy<RwLock<DateTime<Utc>>> =
    Lazy::new(|| RwLock::new(DateTime::<Utc>::MIN_UTC));

#[derive(Debug, Deserialize)]
struct TakerEvent {
    market_id: u64,
    side: bool,
    market_order_id: u64,
    maker: AccountAddress,
    custodian_id: Option<u64>,
    size: u64,
    price: u64,
}

impl From<TakerEvent> for NewTakerEvent {
    fn from(e: TakerEvent) -> Self {
        Self {
            market_id: e.market_id.into(),
            side: e.side.into(),
            market_order_id: e.market_order_id.into(),
            maker: e.maker.to_hex_literal(),
            custodian_id: e.custodian_id.map(|c| c.into()),
            size: e.size.into(),
            price: e.price.into(),
            time: *CURRENT_BLOCK_TIME.read().unwrap(),
        }
    }
}

#[derive(Debug, Deserialize)]
struct MakerEvent {
    market_id: u64,
    side: bool,
    market_order_id: u64,
    user_address: AccountAddress,
    custodian_id: Option<u64>,
    event_type: u8,
    size: u64,
    price: u64,
}

impl From<MakerEvent> for NewMakerEvent {
    fn from(e: MakerEvent) -> Self {
        Self {
            market_id: e.market_id.into(),
            side: e.side.into(),
            market_order_id: e.market_order_id.into(),
            user_address: e.user_address.to_hex_literal(),
            custodian_id: e.custodian_id.map(|c| c.into()),
            event_type: e.event_type.try_into().unwrap(),
            size: e.size.into(),
            price: e.price.into(),
            time: *CURRENT_BLOCK_TIME.read().unwrap(),
        }
    }
}

#[derive(Debug, Deserialize)]
struct MarketRegistrationEvent {
    market_id: u64,
    base_type: TypeInfo,
    base_name_generic: String,
    quote_type: TypeInfo,
    lot_size: u64,
    tick_size: u64,
    min_size: u64,
    underwriter_id: u64,
}

impl From<MarketRegistrationEvent> for NewMarketRegistrationEvent {
    fn from(e: MarketRegistrationEvent) -> Self {
        Self {
            market_id: e.market_id.into(),
            time: *CURRENT_BLOCK_TIME.read().unwrap(),
            base_account_address: Some(e.base_type.account_address),
            base_module_name: Some(e.base_type.module_name),
            base_struct_name: Some(e.base_type.struct_name),
            base_name_generic: Some(e.base_name_generic),
            quote_account_address: e.quote_type.account_address,
            quote_module_name: e.quote_type.module_name,
            quote_struct_name: e.quote_type.struct_name,
            lot_size: e.lot_size.into(),
            tick_size: e.tick_size.into(),
            min_size: e.min_size.into(),
            underwriter_id: e.underwriter_id.into(),
        }
    }
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct TradingPair {
    base_type: TypeInfo,
    base_name_generic: String,
    quote_type: TypeInfo,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct RecognizedMarketInfo {
    market_id: u64,
    lot_size: u64,
    tick_size: u64,
    min_size: u64,
    underwriter_id: u64,
}

#[derive(Debug, Deserialize)]
struct RecognizedMarketEvent {
    trading_pair: TradingPair,
    recognized_market_info: Option<RecognizedMarketInfo>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum EventWrapper {
    Maker(MakerEvent),
    Taker(TakerEvent),
    MarketRegistration(MarketRegistrationEvent),
    RecognizedMarket(RecognizedMarketEvent),
}

pub struct EconiaTransactionProcessor {
    connection_pool: PgDbPool,
}

impl EconiaTransactionProcessor {
    pub fn new(connection_pool: PgDbPool) -> Self {
        Self { connection_pool }
    }
}

fn is_event_type_valid(e: &EventModel) -> bool {
    EVENT_TYPES.iter().find(|t| *t == &e.type_).is_some()
}

impl std::fmt::Debug for EconiaTransactionProcessor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let state = &self.connection_pool.state();
        write!(
            f,
            "EconiaTransactionProcessor {{ connections: {:?}  idle_connections: {:?} }}",
            state.connections, state.idle_connections
        )
    }
}

fn insert_events_transaction(
    conn: &mut PgPoolConnection,
    events: &[EventModel],
) -> Result<(), Error> {
    conn.build_transaction()
        .read_write()
        .run::<_, Error, _>(|pg_conn| {
            insert_events(pg_conn, events)?;
            Ok(())
        })?;
    Ok(())
}

fn insert_to_db(
    conn: &mut PgPoolConnection,
    start_version: u64,
    end_version: u64,
    events: Vec<EventModel>,
) -> Result<ProcessingResult, Error> {
    aptos_logger::trace!(
        name = NAME,
        start_version = start_version,
        end_version = end_version,
        "Inserting to db",
    );
    if insert_events_transaction(conn, &events).is_err() {
        let events = clean_data_for_db(events, true);
        insert_events_transaction(conn, &events)?;
    }
    Ok(ProcessingResult::new(NAME, start_version, end_version))
}

fn insert_events(conn: &mut PgConnection, ev: &[EventModel]) -> Result<(), Error> {
    let mut maker = vec![];
    let mut taker = vec![];
    let mut market_registration = vec![];
    let mut recognized_market = vec![];

    for e in ev.iter() {
        let event_wrapper: EventWrapper = serde_json::from_value(e.data.clone())
            .map_err(|e| Error::DeserializationError(Box::new(e)))?;
        match event_wrapper {
            EventWrapper::Maker(e) => maker.push(e),
            EventWrapper::Taker(e) => taker.push(e),
            EventWrapper::MarketRegistration(e) => market_registration.push(e),
            EventWrapper::RecognizedMarket(e) => recognized_market.push(e),
        }
    }

    insert_maker_events(conn, maker)?;
    insert_taker_events(conn, taker)?;
    insert_market_registration_events(conn, market_registration)?;
    insert_recognized_market_events(conn, recognized_market)?;

    Ok(())
}

fn insert_maker_events(conn: &mut PgConnection, ev: Vec<MakerEvent>) -> Result<(), Error> {
    let ev = ev
        .into_iter()
        .map(|e| models::events::NewMakerEvent::from(e))
        .collect::<Vec<_>>();
    let chunks = get_chunks(ev.len(), models::events::NewMakerEvent::field_count());
    let table = econia_db::schema::maker_events::table;
    for (start_ind, end_ind) in chunks {
        execute_with_better_error(
            conn,
            diesel::insert_into(table)
                .values(&ev[start_ind..end_ind])
                .on_conflict_do_nothing(),
            None,
        )?;
    }
    Ok(())
}

fn insert_taker_events(conn: &mut PgConnection, ev: Vec<TakerEvent>) -> Result<(), Error> {
    let ev = ev
        .into_iter()
        .map(|e| models::events::NewTakerEvent::from(e))
        .collect::<Vec<_>>();
    let chunks = get_chunks(ev.len(), models::events::NewTakerEvent::field_count());
    let table = econia_db::schema::taker_events::table;
    for (start_ind, end_ind) in chunks {
        execute_with_better_error(
            conn,
            diesel::insert_into(table)
                .values(&ev[start_ind..end_ind])
                .on_conflict_do_nothing(),
            None,
        )?;
    }
    Ok(())
}

fn insert_market_registration_events(
    conn: &mut PgConnection,
    ev: Vec<MarketRegistrationEvent>,
) -> Result<(), Error> {
    let ev = ev
        .into_iter()
        .map(|e| models::market::NewMarketRegistrationEvent::from(e))
        .collect::<Vec<_>>();
    let chunks = get_chunks(
        ev.len(),
        models::market::NewMarketRegistrationEvent::field_count(),
    );
    let table = econia_db::schema::market_registration_events::table;
    for (start_ind, end_ind) in chunks {
        execute_with_better_error(
            conn,
            diesel::insert_into(table)
                .values(&ev[start_ind..end_ind])
                .on_conflict_do_nothing(),
            None,
        )?;
    }
    Ok(())
}

// TODO cache recognized markets on startup rather than doing this query below
fn insert_recognized_market_events(
    conn: &mut PgConnection,
    ev: Vec<RecognizedMarketEvent>,
) -> Result<(), Error> {
    use diesel::prelude::*;
    use econia_db::schema::market_registration_events::dsl::*;

    let mut events = vec![];

    for e in ev.into_iter() {
        let market = market_registration_events
            .filter(base_account_address.eq(e.trading_pair.base_type.account_address))
            .filter(base_module_name.eq(e.trading_pair.base_type.module_name))
            .filter(base_struct_name.eq(e.trading_pair.base_type.struct_name))
            .filter(quote_account_address.eq(e.trading_pair.quote_type.account_address))
            .filter(quote_module_name.eq(e.trading_pair.quote_type.module_name))
            .filter(quote_struct_name.eq(e.trading_pair.quote_type.struct_name))
            .load::<models::market::MarketRegistrationEvent>(conn)?;

        let mkt = &market[0];
        if let Some(r) = e.recognized_market_info {
            let new_lot_size = BigDecimal::from(r.lot_size);
            let new_tick_size = BigDecimal::from(r.tick_size);
            let new_min_size = BigDecimal::from(r.min_size);
            events.push(NewRecognizedMarketEvent {
                market_id: mkt.market_id.clone(),
                time: *CURRENT_BLOCK_TIME.read().unwrap(),
                event_type: if mkt.lot_size == new_lot_size
                    && mkt.tick_size == new_tick_size
                    && mkt.min_size == new_min_size
                {
                    MarketEventType::Add
                } else {
                    MarketEventType::Update
                },
                lot_size: Some(new_lot_size),
                tick_size: Some(new_tick_size),
                min_size: Some(new_min_size),
            })
        } else {
            events.push(NewRecognizedMarketEvent {
                market_id: mkt.market_id.clone(),
                time: *CURRENT_BLOCK_TIME.read().unwrap(),
                event_type: MarketEventType::Remove,
                lot_size: None,
                tick_size: None,
                min_size: None,
            })
        }
    }

    let chunks = get_chunks(
        events.len(),
        models::market::NewRecognizedMarketEvent::field_count(),
    );
    let table = econia_db::schema::recognized_market_events::table;
    for (start_ind, end_ind) in chunks {
        execute_with_better_error(
            conn,
            diesel::insert_into(table)
                .values(&events[start_ind..end_ind])
                .on_conflict_do_nothing(),
            None,
        )?;
    }
    Ok(())
}

#[async_trait]
impl TransactionProcessor for EconiaTransactionProcessor {
    fn name(&self) -> &'static str {
        NAME
    }

    async fn process_transactions(
        &self,
        transactions: Vec<Transaction>,
        start_version: u64,
        end_version: u64,
    ) -> Result<ProcessingResult, TransactionProcessingError> {
        let (_, _, events, _, _) = TransactionModel::from_transactions(&transactions);
        let events = events
            .into_iter()
            .filter(is_event_type_valid)
            .collect::<Vec<EventModel>>();

        let mut conn = self.get_conn();
        insert_to_db(&mut conn, start_version, end_version, events).map_err(|err| {
            TransactionProcessingError::TransactionCommitError((
                anyhow::Error::from(err),
                start_version,
                end_version,
                NAME,
            ))
        })
    }

    fn connection_pool(&self) -> &PgDbPool {
        &self.connection_pool
    }
}
