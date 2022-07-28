mod transaction_queries;

use aptos_indexer::{
    database::PgDbPool,
    models::{
        events::Event,
        transactions::{BlockMetadataTransaction, Transaction, UserTransaction},
        write_set_changes::WriteSetChange,
    },
};

use async_graphql::{Context, FieldResult, Object, SimpleObject};

pub struct QueryRoot;
pub struct ContextData {
    pub pool: PgDbPool,
}

#[derive(SimpleObject)]
pub struct AptosTransaction {
    transaction_info: Transaction,
    block_metadata_transaction: Option<BlockMetadataTransaction>,
    user_transaction: Option<UserTransaction>,
    events: Vec<Event>,
    writesets: Vec<WriteSetChange>,
}

#[Object]
impl QueryRoot {
    pub async fn get_transaction_by_version(
        &self,
        ctx: &Context<'_>,
        version: u64,
    ) -> FieldResult<AptosTransaction> {
        let data = ctx.data::<ContextData>()?;
        let conn = &data.pool.get().unwrap();
        transaction_queries::get_transaction_by_version(version, conn)
    }

    pub async fn get_transactions_by_start_version(
        &self,
        ctx: &Context<'_>,
        start_version: i64,
        limit: i64,
    ) -> FieldResult<Vec<AptosTransaction>> {
        let data = ctx.data::<ContextData>()?;
        let conn = &data.pool.get().unwrap();
        transaction_queries::get_transactions_by_start_version(start_version, limit, conn)
    }

    pub async fn get_transactions_by_block(
        &self,
        ctx: &Context<'_>,
        block_height: i64,
    ) -> FieldResult<Vec<AptosTransaction>> {
        let data = ctx.data::<ContextData>()?;
        let conn = &data.pool.get().unwrap();
        transaction_queries::get_transactions_by_block(block_height, conn)
    }
}
