use std::{collections::HashMap, sync::RwLock};

use crate::{
    database::{clean_data_for_db, execute_with_better_error, get_chunks, PgDbPool},
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

impl From<&MarketRegistrationEvent> for models::market::MarketRegistrationEvent {
    fn from(value: &MarketRegistrationEvent) -> Self {
        Self {
            market_id: value.market_id.into(),
            time: *CURRENT_BLOCK_TIME.read().unwrap(),
            base_account_address: Some(value.base_type.account_address.clone()),
            base_module_name: Some(value.base_type.module_name.clone()),
            base_struct_name: Some(value.base_type.struct_name.clone()),
            base_name_generic: Some(value.base_name_generic.clone()),
            quote_account_address: value.quote_type.account_address.clone(),
            quote_module_name: value.quote_type.module_name.clone(),
            quote_struct_name: value.quote_type.struct_name.clone(),
            lot_size: value.lot_size.into(),
            tick_size: value.tick_size.into(),
            min_size: value.min_size.into(),
            underwriter_id: value.underwriter_id.into(),
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

fn is_event_type_valid(e: &EventModel) -> bool {
    EVENT_TYPES.iter().any(|t| t == &e.type_)
}

fn fetch_all_markets(
    conn: &mut PgConnection,
) -> Result<Vec<models::market::MarketRegistrationEvent>, Error> {
    use diesel::prelude::*;
    use econia_db::schema::market_registration_events::dsl::*;
    market_registration_events.load::<models::market::MarketRegistrationEvent>(conn)
}

type BaseQuoteKey = (String, String, String, String, String, String);

fn create_base_quote_key(m: &models::market::MarketRegistrationEvent) -> BaseQuoteKey {
    (
        m.base_account_address.clone().unwrap_or_default(),
        m.base_module_name.clone().unwrap_or_default(),
        m.base_struct_name.clone().unwrap_or_default(),
        m.quote_account_address.clone(),
        m.quote_module_name.clone(),
        m.quote_struct_name.clone(),
    )
}

pub struct EconiaTransactionProcessor {
    connection_pool: PgDbPool,
    markets: RwLock<HashMap<BigDecimal, models::market::MarketRegistrationEvent>>,
    base_quote_to_market_id: RwLock<HashMap<BaseQuoteKey, BigDecimal>>,
}

impl EconiaTransactionProcessor {
    pub fn new(connection_pool: PgDbPool) -> Self {
        let mut conn = connection_pool
            .get()
            .expect("failed connecting to db on startup");
        let mkts = fetch_all_markets(&mut conn).expect("failed loading markets on startup");
        let mut markets = HashMap::new();
        let mut base_quote_to_market_id = HashMap::new();
        for m in mkts.into_iter() {
            let key = create_base_quote_key(&m);
            base_quote_to_market_id.insert(key, m.market_id.clone());
            markets.insert(m.market_id.clone(), m);
        }

        Self {
            connection_pool,
            markets: RwLock::new(markets),
            base_quote_to_market_id: RwLock::new(base_quote_to_market_id),
        }
    }

    fn update_markets_cache(&self, ev: &[MarketRegistrationEvent]) {
        for e in ev.iter() {
            let m = models::market::MarketRegistrationEvent::from(e);
            let key = create_base_quote_key(&m);
            self.base_quote_to_market_id
                .write()
                .unwrap()
                .insert(key, m.market_id.clone());
            self.markets.write().unwrap().insert(m.market_id.clone(), m);
        }
    }

    fn insert_to_db(
        &self,
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
        if self.insert_events_transaction(&events).is_err() {
            let events = clean_data_for_db(events, true);
            self.insert_events_transaction(&events)?;
        }
        Ok(ProcessingResult::new(NAME, start_version, end_version))
    }

    fn insert_events_transaction(&self, events: &[EventModel]) -> Result<(), Error> {
        let mut conn = self.get_conn();
        conn.build_transaction()
            .read_write()
            .run::<_, Error, _>(|pg_conn| {
                self.insert_events(pg_conn, events)?;
                Ok(())
            })?;
        Ok(())
    }

    fn insert_events(&self, conn: &mut PgConnection, ev: &[EventModel]) -> Result<(), Error> {
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

        self.insert_maker_events(conn, maker)?;
        self.insert_taker_events(conn, taker)?;

        // update markets cache
        self.update_markets_cache(&market_registration);

        self.insert_market_registration_events(conn, market_registration)?;
        self.insert_recognized_market_events(conn, recognized_market)?;
        Ok(())
    }

    fn insert_maker_events(
        &self,
        conn: &mut PgConnection,
        ev: Vec<MakerEvent>,
    ) -> Result<(), Error> {
        let ev = ev
            .into_iter()
            .map(models::events::NewMakerEvent::from)
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

    fn insert_taker_events(
        &self,
        conn: &mut PgConnection,
        ev: Vec<TakerEvent>,
    ) -> Result<(), Error> {
        let ev = ev
            .into_iter()
            .map(models::events::NewTakerEvent::from)
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
        &self,
        conn: &mut PgConnection,
        ev: Vec<MarketRegistrationEvent>,
    ) -> Result<(), Error> {
        let ev = ev
            .into_iter()
            .map(models::market::NewMarketRegistrationEvent::from)
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

    fn convert_recognized_market_events_to_db(
        &self,
        ev: Vec<RecognizedMarketEvent>,
    ) -> Result<Vec<NewRecognizedMarketEvent>, Error> {
        let mut events = vec![];
        for e in ev.into_iter() {
            if let Some(r) = e.recognized_market_info {
                let mkt = self.markets.read().unwrap();
                let mkt = mkt.get(&r.market_id.into()).ok_or(Error::NotFound)?;
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
                let key: BaseQuoteKey = (
                    e.trading_pair.base_type.account_address,
                    e.trading_pair.base_type.module_name,
                    e.trading_pair.base_type.struct_name,
                    e.trading_pair.quote_type.account_address,
                    e.trading_pair.quote_type.module_name,
                    e.trading_pair.quote_type.struct_name,
                );
                let mkt_id = self.base_quote_to_market_id.read().unwrap();
                let mkt_id = mkt_id.get(&key).unwrap();
                events.push(NewRecognizedMarketEvent {
                    market_id: mkt_id.clone(),
                    time: *CURRENT_BLOCK_TIME.read().unwrap(),
                    event_type: MarketEventType::Remove,
                    lot_size: None,
                    tick_size: None,
                    min_size: None,
                })
            }
        }
        Ok(events)
    }

    fn insert_recognized_market_events(
        &self,
        conn: &mut PgConnection,
        ev: Vec<RecognizedMarketEvent>,
    ) -> Result<(), Error> {
        let events = self.convert_recognized_market_events_to_db(ev)?;
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

        self.insert_to_db(start_version, end_version, events)
            .map_err(|err| {
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
