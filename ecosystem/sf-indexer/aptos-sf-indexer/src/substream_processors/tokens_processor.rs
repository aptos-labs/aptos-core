// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    database::{execute_with_better_error, get_chunks, PgDbPool, PgPoolConnection},
    indexer::{
        errors::BlockProcessingError,
        processing_result::ProcessingResult,
        substream_processor::{get_conn, SubstreamProcessor},
    },
    models::tokens::{CollectionData, Token, TokenData, TokenOwnership},
    proto::{module_output::Data as ModuleOutputData, BlockScopedData},
    schema,
};
use anyhow::format_err;
use aptos_protos::tokens::v1::Tokens;
use async_trait::async_trait;
use diesel::result::Error;
use field_count::FieldCount;
use prost::Message;
use std::fmt::Debug;

pub struct TokensSubstreamProcessor {
    connection_pool: PgDbPool,
    is_chain_id_verified: bool,
}

impl TokensSubstreamProcessor {
    pub fn new(connection_pool: PgDbPool) -> Self {
        Self {
            connection_pool,
            is_chain_id_verified: false,
        }
    }
}

impl Debug for TokensSubstreamProcessor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let state = &self.connection_pool.state();
        write!(
            f,
            "TokensSubstreamProcessor {{ connections: {:?}  idle_connections: {:?} }}",
            state.connections, state.idle_connections
        )
    }
}

/// This function handles tokens in a block
fn handle_tokens_in_block(
    conn: &mut PgPoolConnection,
    substream_name: &'static str,
    block_height: u64,
    tokens: Vec<Token>,
    token_datas: Vec<TokenData>,
    token_ownerships: Vec<TokenOwnership>,
    collection_datas: Vec<CollectionData>,
) -> Result<(), Error> {
    aptos_logger::trace!("[{}] inserting block {}", substream_name, block_height);
    conn.build_transaction()
        .read_write()
        .run::<_, Error, _>(|| {
            insert_tokens(conn, &tokens);
            insert_token_datas(conn, &token_datas);
            insert_token_ownerships(conn, &token_ownerships);
            insert_collection_datas(conn, &collection_datas);
            Ok(())
        })
}

fn insert_tokens(conn: &mut PgPoolConnection, tokens_to_insert: &[Token]) {
    use schema::tokens::dsl::*;

    let chunks = get_chunks(tokens_to_insert.len(), Token::field_count());
    for (start_ind, end_ind) in chunks {
        execute_with_better_error(
            conn,
            diesel::insert_into(schema::tokens::table)
                .values(&tokens_to_insert[start_ind..end_ind])
                .on_conflict((
                    creator_address,
                    collection_name,
                    name,
                    property_version,
                    transaction_version,
                ))
                .do_nothing(),
        )
        .expect("Error inserting tokens into database");
    }
}

fn insert_token_datas(conn: &mut PgPoolConnection, token_datas_to_insert: &[TokenData]) {
    use schema::token_datas::dsl::*;

    let chunks = get_chunks(token_datas_to_insert.len(), TokenData::field_count());
    for (start_ind, end_ind) in chunks {
        execute_with_better_error(
            conn,
            diesel::insert_into(schema::token_datas::table)
                .values(&token_datas_to_insert[start_ind..end_ind])
                .on_conflict((creator_address, collection_name, name, transaction_version))
                .do_nothing(),
        )
        .expect("Error inserting token_datas into database");
    }
}

fn insert_token_ownerships(
    conn: &mut PgPoolConnection,
    token_ownerships_to_insert: &[TokenOwnership],
) {
    use schema::token_ownerships::dsl::*;

    let chunks = get_chunks(
        token_ownerships_to_insert.len(),
        TokenOwnership::field_count(),
    );
    for (start_ind, end_ind) in chunks {
        execute_with_better_error(
            conn,
            diesel::insert_into(schema::token_ownerships::table)
                .values(&token_ownerships_to_insert[start_ind..end_ind])
                .on_conflict((
                    creator_address,
                    collection_name,
                    name,
                    property_version,
                    transaction_version,
                    table_handle,
                ))
                .do_nothing(),
        )
        .expect("Error inserting token_ownerships into database");
    }
}

fn insert_collection_datas(
    conn: &mut PgPoolConnection,
    collection_datas_to_insert: &[CollectionData],
) {
    use schema::collection_datas::dsl::*;

    let chunks = get_chunks(
        collection_datas_to_insert.len(),
        CollectionData::field_count(),
    );
    for (start_ind, end_ind) in chunks {
        execute_with_better_error(
            conn,
            diesel::insert_into(schema::collection_datas::table)
                .values(&collection_datas_to_insert[start_ind..end_ind])
                .on_conflict((creator_address, collection_name, transaction_version))
                .do_nothing(),
        )
        .expect("Error inserting collection_datas into database");
    }
}

#[async_trait]
impl SubstreamProcessor for TokensSubstreamProcessor {
    fn substream_module_name(&self) -> &'static str {
        "block_output_to_token"
    }

    fn is_chain_id_verified(&self) -> bool {
        self.is_chain_id_verified
    }

    fn set_is_chain_id_verified(&mut self) {
        self.is_chain_id_verified = true;
    }

    async fn process_substream(
        &mut self,
        stream_data: BlockScopedData,
        block_height: u64,
    ) -> Result<ProcessingResult, BlockProcessingError> {
        let output = stream_data.outputs.first().ok_or_else(|| {
            BlockProcessingError::ParsingError((
                format_err!("expecting one module output"),
                block_height,
                self.substream_module_name(),
            ))
        })?;
        // This is the expected output of the substream
        let token_pb: Tokens = match output.data.as_ref().unwrap() {
            ModuleOutputData::MapOutput(data) => {
                aptos_logger::debug!(block_height = block_height, "Parsing mapper for block");
                Message::decode(data.value.as_slice()).map_err(|err| {
                    BlockProcessingError::ParsingError((
                        anyhow::Error::from(err),
                        block_height,
                        self.substream_module_name(),
                    ))
                })?
            }
            ModuleOutputData::StoreDeltas(_) => {
                return Err(BlockProcessingError::ParsingError((
                    format_err!("invalid module output StoreDeltas, expecting MapOutput"),
                    block_height,
                    self.substream_module_name(),
                )));
            }
        };

        if block_height != token_pb.block_height {
            panic!(
                "We should be on block {}, but received block {}",
                block_height, token_pb.block_height
            );
        }
        if !self.is_chain_id_verified() {
            let input_chain_id = token_pb.chain_id;
            self.check_or_update_chain_id(input_chain_id as i64);
        }

        let (tokens, token_datas, token_ownerships, collection_datas) =
            Token::from_tokens(&token_pb);
        let conn = get_conn(self.connection_pool());

        let tx_result = handle_tokens_in_block(
            &conn,
            self.substream_module_name(),
            block_height,
            tokens,
            token_datas,
            token_ownerships,
            collection_datas,
        );

        match tx_result {
            Ok(_) => Ok(ProcessingResult::new(
                self.substream_module_name(),
                block_height,
            )),
            Err(err) => Err(BlockProcessingError::BlockCommitError((
                anyhow::Error::from(err),
                block_height,
                self.substream_module_name(),
            ))),
        }
    }

    fn connection_pool(&self) -> &PgDbPool {
        &self.connection_pool
    }
}
