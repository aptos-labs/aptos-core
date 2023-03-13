// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use super::processor_trait::{ProcessingResult, ProcessorTrait};
use crate::{
    models::stake_models::{
        delegator_activities::DelegatedStakingActivity,
        delegator_balances::{CurrentDelegatorBalance, CurrentDelegatorBalanceMap},
        proposal_votes::ProposalVote,
        staking_pool_voter::{CurrentStakingPoolVoter, StakingPoolVoterMap},
    },
    schema,
    utils::database::{
        clean_data_for_db, execute_with_better_error, get_chunks, PgDbPool, PgPoolConnection,
    },
};
use anyhow::bail;
use aptos_logger::error;
use aptos_protos::transaction::testing1::v1::Transaction;
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
    delegator_actvities: &[DelegatedStakingActivity],
    delegator_balances: &[CurrentDelegatorBalance],
) -> Result<(), diesel::result::Error> {
    insert_current_stake_pool_voter(conn, current_stake_pool_voters)?;
    insert_proposal_votes(conn, proposal_votes)?;
    insert_delegator_activities(conn, delegator_actvities)?;
    insert_delegator_balances(conn, delegator_balances)?;
    Ok(())
}

fn insert_to_db(
    conn: &mut PgPoolConnection,
    name: &'static str,
    start_version: u64,
    end_version: u64,
    current_stake_pool_voters: Vec<CurrentStakingPoolVoter>,
    proposal_votes: Vec<ProposalVote>,
    delegator_actvities: Vec<DelegatedStakingActivity>,
    delegator_balances: Vec<CurrentDelegatorBalance>,
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
            insert_to_db_impl(
                pg_conn,
                &current_stake_pool_voters,
                &proposal_votes,
                &delegator_actvities,
                &delegator_balances,
            )
        }) {
        Ok(_) => Ok(()),
        Err(_) => conn
            .build_transaction()
            .read_write()
            .run::<_, Error, _>(|pg_conn| {
                let current_stake_pool_voters = clean_data_for_db(current_stake_pool_voters, true);
                let proposal_votes = clean_data_for_db(proposal_votes, true);
                let delegator_actvities = clean_data_for_db(delegator_actvities, true);
                let delegator_balances = clean_data_for_db(delegator_balances, true);

                insert_to_db_impl(
                    pg_conn,
                    &current_stake_pool_voters,
                    &proposal_votes,
                    &delegator_actvities,
                    &delegator_balances,
                )
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

fn insert_delegator_activities(
    conn: &mut PgConnection,
    item_to_insert: &[DelegatedStakingActivity],
) -> Result<(), diesel::result::Error> {
    use schema::delegated_staking_activities::dsl::*;

    let chunks = get_chunks(
        item_to_insert.len(),
        DelegatedStakingActivity::field_count(),
    );
    for (start_ind, end_ind) in chunks {
        execute_with_better_error(
            conn,
            diesel::insert_into(schema::delegated_staking_activities::table)
                .values(&item_to_insert[start_ind..end_ind])
                .on_conflict((transaction_version, event_index))
                .do_nothing(),
            None,
        )?;
    }
    Ok(())
}

fn insert_delegator_balances(
    conn: &mut PgConnection,
    item_to_insert: &[CurrentDelegatorBalance],
) -> Result<(), diesel::result::Error> {
    use schema::current_delegator_balances::dsl::*;

    let chunks = get_chunks(item_to_insert.len(), CurrentDelegatorBalance::field_count());
    for (start_ind, end_ind) in chunks {
        execute_with_better_error(
            conn,
            diesel::insert_into(schema::current_delegator_balances::table)
                .values(&item_to_insert[start_ind..end_ind])
                .on_conflict((delegator_address, pool_address, pool_type))
                .do_update()
                .set((
                    table_handle.eq(excluded(table_handle)),
                    amount.eq(excluded(amount)),
                    last_transaction_version.eq(excluded(last_transaction_version)),
                    inserted_at.eq(excluded(inserted_at)),
                )),
            Some(
                " WHERE current_delegator_balances.last_transaction_version <= EXCLUDED.last_transaction_version ",
            ),
        )?;
    }
    Ok(())
}

#[async_trait]
impl ProcessorTrait for StakeTransactionProcessor {
    fn name(&self) -> &'static str {
        NAME
    }

    async fn process_transactions(
        &self,
        transactions: Vec<Transaction>,
        start_version: u64,
        end_version: u64,
    ) -> anyhow::Result<ProcessingResult> {
        let mut all_current_stake_pool_voters: StakingPoolVoterMap = HashMap::new();
        let mut all_proposal_votes = vec![];
        let mut all_delegator_activities = vec![];
        let mut all_delegator_balances: CurrentDelegatorBalanceMap = HashMap::new();

        for txn in &transactions {
            // Add votes data
            let current_stake_pool_voter = CurrentStakingPoolVoter::from_transaction(txn).unwrap();
            all_current_stake_pool_voters.extend(current_stake_pool_voter);
            let mut proposal_votes = ProposalVote::from_transaction(txn).unwrap();
            all_proposal_votes.append(&mut proposal_votes);

            // Add delegator activities
            let mut delegator_activities = DelegatedStakingActivity::from_transaction(txn).unwrap();
            all_delegator_activities.append(&mut delegator_activities);

            // Add delegator balances
            let delegator_balances = CurrentDelegatorBalance::from_transaction(txn).unwrap();
            all_delegator_balances.extend(delegator_balances);
        }

        // Getting list of values and sorting by pk in order to avoid postgres deadlock since we're doing multi threaded db writes
        let mut all_current_stake_pool_voters = all_current_stake_pool_voters
            .into_values()
            .collect::<Vec<CurrentStakingPoolVoter>>();
        let mut all_delegator_balances = all_delegator_balances
            .into_values()
            .collect::<Vec<CurrentDelegatorBalance>>();

        // Sort by PK
        all_current_stake_pool_voters
            .sort_by(|a, b| a.staking_pool_address.cmp(&b.staking_pool_address));

        // Sort by PK
        all_delegator_balances.sort_by(|a, b| {
            (&a.delegator_address, &a.pool_address, &a.pool_type).cmp(&(
                &b.delegator_address,
                &b.pool_address,
                &b.pool_type,
            ))
        });

        let mut conn = self.get_conn();
        let tx_result = insert_to_db(
            &mut conn,
            self.name(),
            start_version,
            end_version,
            all_current_stake_pool_voters,
            all_proposal_votes,
            all_delegator_activities,
            all_delegator_balances,
        );
        match tx_result {
            Ok(_) => Ok((start_version, end_version)),
            Err(e) => {
                error!(
                    start_version = start_version,
                    end_version = end_version,
                    processor_name = self.name(),
                    error = ?e,
                    "[Parser] Error inserting transactions to db",
                );
                bail!(e)
            },
        }
    }

    fn connection_pool(&self) -> &PgDbPool {
        &self.connection_pool
    }
}
