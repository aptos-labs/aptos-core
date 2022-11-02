// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    database::{
        clean_data_for_db, execute_with_better_error, get_chunks, PgDbPool, PgPoolConnection,
    },
    indexer::{
        errors::TransactionProcessingError, processing_result::ProcessingResult,
        transaction_processor::TransactionProcessor,
    },
    models::stake_models::{
        proposal_votes::ProposalVote,
        staking_pool_voter::{CurrentStakingPoolVoter, StakingPoolVoterMap},
    },
    schema,
};
use aptos_api_types::Transaction as APITransaction;
use async_trait::async_trait;
use diesel::{pg::upsert::excluded, result::Error, ExpressionMethods, PgConnection};
use field_count::FieldCount;
use std::{collections::HashMap, fmt::Debug};

pub const NAME: &str = "stake_processor";
pub struct StakeTransactionProcessor {
    connection_pool: PgDbPool,
}

impl StakeTransactionProcessor {
    pub fn new(connection_pool: PgDbPool) -> Self {
        Self { connection_pool }
    }
}

impl Debug for StakeTransactionProcessor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let state = &self.connection_pool.state();
        write!(
            f,
            "StakeTransactionProcessor {{ connections: {:?}  idle_connections: {:?} }}",
            state.connections, state.idle_connections
        )
    }
}

fn insert_to_db_impl(
    conn: &mut PgConnection,
    current_stake_pool_voters: &[CurrentStakingPoolVoter],
    proposal_votes: &[ProposalVote],
) -> Result<(), diesel::result::Error> {
    insert_current_stake_pool_voter(conn, current_stake_pool_voters)?;
    insert_proposal_votes(conn, proposal_votes)?;
    Ok(())
}

fn insert_to_db(
    conn: &mut PgPoolConnection,
    name: &'static str,
    start_version: u64,
    end_version: u64,
    current_stake_pool_voters: Vec<CurrentStakingPoolVoter>,
    proposal_votes: Vec<ProposalVote>,
) -> Result<(), diesel::result::Error> {
    aptos_logger::trace!(
        name = name,
        start_version = start_version,
        end_version = end_version,
        "Inserting to db",
    );
    match conn
        .build_transaction()
        .read_write()
        .run::<_, Error, _>(|pg_conn| {
            insert_to_db_impl(pg_conn, &current_stake_pool_voters, &proposal_votes)
        }) {
        Ok(_) => Ok(()),
        Err(_) => conn
            .build_transaction()
            .read_write()
            .run::<_, Error, _>(|pg_conn| {
                let current_stake_pool_voters = clean_data_for_db(current_stake_pool_voters, true);
                let proposal_votes = clean_data_for_db(proposal_votes, true);

                insert_to_db_impl(pg_conn, &current_stake_pool_voters, &proposal_votes)
            }),
    }
}

fn insert_current_stake_pool_voter(
    conn: &mut PgConnection,
    item_to_insert: &[CurrentStakingPoolVoter],
) -> Result<(), diesel::result::Error> {
    use schema::current_staking_pool_voter::dsl::*;

    let chunks = get_chunks(item_to_insert.len(), CurrentStakingPoolVoter::field_count());
    for (start_ind, end_ind) in chunks {
        execute_with_better_error(
            conn,
            diesel::insert_into(schema::current_staking_pool_voter::table)
                .values(&item_to_insert[start_ind..end_ind])
                .on_conflict(staking_pool_address)
                .do_update()
                .set((
                    staking_pool_address.eq(excluded(staking_pool_address)),
                    voter_address.eq(excluded(voter_address)),
                    last_transaction_version.eq(excluded(last_transaction_version)),
                    inserted_at.eq(excluded(inserted_at)),
                )),
            Some(
                " WHERE current_staking_pool_voter.last_transaction_version <= EXCLUDED.last_transaction_version ",
            ),
        )?;
    }
    Ok(())
}

fn insert_proposal_votes(
    conn: &mut PgConnection,
    item_to_insert: &[ProposalVote],
) -> Result<(), diesel::result::Error> {
    use schema::proposal_votes::dsl::*;

    let chunks = get_chunks(item_to_insert.len(), ProposalVote::field_count());
    for (start_ind, end_ind) in chunks {
        execute_with_better_error(
            conn,
            diesel::insert_into(schema::proposal_votes::table)
                .values(&item_to_insert[start_ind..end_ind])
                .on_conflict((transaction_version, proposal_id, voter_address))
                .do_nothing(),
            None,
        )?;
    }
    Ok(())
}

#[async_trait]
impl TransactionProcessor for StakeTransactionProcessor {
    fn name(&self) -> &'static str {
        NAME
    }

    async fn process_transactions(
        &self,
        transactions: Vec<APITransaction>,
        start_version: u64,
        end_version: u64,
    ) -> Result<ProcessingResult, TransactionProcessingError> {
        let mut all_current_stake_pool_voters: StakingPoolVoterMap = HashMap::new();
        let mut all_proposal_votes = vec![];

        for txn in &transactions {
            let current_stake_pool_voter = CurrentStakingPoolVoter::from_transaction(txn).unwrap();
            all_current_stake_pool_voters.extend(current_stake_pool_voter);
            let mut proposal_votes = ProposalVote::from_transaction(txn).unwrap();
            all_proposal_votes.append(&mut proposal_votes);
        }
        let mut all_current_stake_pool_voters = all_current_stake_pool_voters
            .into_values()
            .collect::<Vec<CurrentStakingPoolVoter>>();

        // Sort by PK
        all_current_stake_pool_voters
            .sort_by(|a, b| a.staking_pool_address.cmp(&b.staking_pool_address));

        let mut conn = self.get_conn();
        let tx_result = insert_to_db(
            &mut conn,
            self.name(),
            start_version,
            end_version,
            all_current_stake_pool_voters,
            all_proposal_votes,
        );
        match tx_result {
            Ok(_) => Ok(ProcessingResult::new(
                self.name(),
                start_version,
                end_version,
            )),
            Err(err) => Err(TransactionProcessingError::TransactionCommitError((
                anyhow::Error::from(err),
                start_version,
                end_version,
                self.name(),
            ))),
        }
    }

    fn connection_pool(&self) -> &PgDbPool {
        &self.connection_pool
    }
}
