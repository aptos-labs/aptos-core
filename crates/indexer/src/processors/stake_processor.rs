// Copyright © Velor Foundation
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
        delegator_activities::DelegatedStakingActivity,
        delegator_balances::{CurrentDelegatorBalance, CurrentDelegatorBalanceMap},
        delegator_pools::{
            CurrentDelegatorPoolBalance, DelegatorPool, DelegatorPoolBalance, DelegatorPoolMap,
        },
        proposal_votes::ProposalVote,
        staking_pool_voter::{CurrentStakingPoolVoter, StakingPoolVoterMap},
    },
    schema,
};
use velor_api_types::Transaction as APITransaction;
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
    delegator_pools: &[DelegatorPool],
    delegator_pool_balances: &[DelegatorPoolBalance],
    current_delegator_pool_balances: &[CurrentDelegatorPoolBalance],
) -> Result<(), diesel::result::Error> {
    insert_current_stake_pool_voter(conn, current_stake_pool_voters)?;
    insert_proposal_votes(conn, proposal_votes)?;
    insert_delegator_activities(conn, delegator_actvities)?;
    insert_delegator_balances(conn, delegator_balances)?;
    insert_delegator_pools(conn, delegator_pools)?;
    insert_delegator_pool_balances(conn, delegator_pool_balances)?;
    insert_current_delegator_pool_balances(conn, current_delegator_pool_balances)?;
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
    delegator_pools: Vec<DelegatorPool>,
    delegator_pool_balances: Vec<DelegatorPoolBalance>,
    current_delegator_pool_balances: Vec<CurrentDelegatorPoolBalance>,
) -> Result<(), diesel::result::Error> {
    velor_logger::trace!(
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
                &delegator_pools,
                &delegator_pool_balances,
                &current_delegator_pool_balances,
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
                let delegator_pools = clean_data_for_db(delegator_pools, true);
                let delegator_pool_balances = clean_data_for_db(delegator_pool_balances, true);
                let current_delegator_pool_balances =
                    clean_data_for_db(current_delegator_pool_balances, true);

                insert_to_db_impl(
                    pg_conn,
                    &current_stake_pool_voters,
                    &proposal_votes,
                    &delegator_actvities,
                    &delegator_balances,
                    &delegator_pools,
                    &delegator_pool_balances,
                    &current_delegator_pool_balances,
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
                    operator_address.eq(excluded(operator_address)),
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
                .on_conflict((delegator_address, pool_address, pool_type, table_handle))
                .do_update()
                .set((
                    last_transaction_version.eq(excluded(last_transaction_version)),
                    inserted_at.eq(excluded(inserted_at)),
                    shares.eq(excluded(shares)),
                    parent_table_handle.eq(excluded(parent_table_handle)),
                )),
            Some(
                " WHERE current_delegator_balances.last_transaction_version <= EXCLUDED.last_transaction_version ",
            ),
        )?;
    }
    Ok(())
}

fn insert_delegator_pools(
    conn: &mut PgConnection,
    item_to_insert: &[DelegatorPool],
) -> Result<(), diesel::result::Error> {
    use schema::delegated_staking_pools::dsl::*;

    let chunks = get_chunks(item_to_insert.len(), DelegatorPool::field_count());
    for (start_ind, end_ind) in chunks {
        execute_with_better_error(
            conn,
            diesel::insert_into(schema::delegated_staking_pools::table)
                .values(&item_to_insert[start_ind..end_ind])
                .on_conflict(staking_pool_address)
                .do_update()
                .set((
                    first_transaction_version.eq(excluded(first_transaction_version)),
                    inserted_at.eq(excluded(inserted_at)),
                )),
            Some(
                " WHERE delegated_staking_pools.first_transaction_version >= EXCLUDED.first_transaction_version ",
            ),
        )?;
    }
    Ok(())
}

fn insert_delegator_pool_balances(
    conn: &mut PgConnection,
    item_to_insert: &[DelegatorPoolBalance],
) -> Result<(), diesel::result::Error> {
    use schema::delegated_staking_pool_balances::dsl::*;

    let chunks = get_chunks(item_to_insert.len(), DelegatorPoolBalance::field_count());
    for (start_ind, end_ind) in chunks {
        execute_with_better_error(
            conn,
            diesel::insert_into(schema::delegated_staking_pool_balances::table)
                .values(&item_to_insert[start_ind..end_ind])
                .on_conflict((transaction_version, staking_pool_address))
                .do_nothing(),
            None,
        )?;
    }
    Ok(())
}

fn insert_current_delegator_pool_balances(
    conn: &mut PgConnection,
    item_to_insert: &[CurrentDelegatorPoolBalance],
) -> Result<(), diesel::result::Error> {
    use schema::current_delegated_staking_pool_balances::dsl::*;

    let chunks = get_chunks(
        item_to_insert.len(),
        CurrentDelegatorPoolBalance::field_count(),
    );
    for (start_ind, end_ind) in chunks {
        execute_with_better_error(
            conn,
            diesel::insert_into(schema::current_delegated_staking_pool_balances::table)
                .values(&item_to_insert[start_ind..end_ind])
                .on_conflict(staking_pool_address)
                .do_update()
                .set((
                    total_coins.eq(excluded(total_coins)),
                    total_shares.eq(excluded(total_shares)),
                    last_transaction_version.eq(excluded(last_transaction_version)),
                    inserted_at.eq(excluded(inserted_at)),
                    operator_commission_percentage.eq(excluded(operator_commission_percentage)),
                    inactive_table_handle.eq(excluded(inactive_table_handle)),
                    active_table_handle.eq(excluded(active_table_handle)),
                )),
            Some(
                " WHERE current_delegated_staking_pool_balances.last_transaction_version <= EXCLUDED.last_transaction_version ",
            ),
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
        let mut conn = self.get_conn();

        let mut all_current_stake_pool_voters: StakingPoolVoterMap = HashMap::new();
        let mut all_proposal_votes = vec![];
        let mut all_delegator_activities = vec![];
        let mut all_delegator_balances: CurrentDelegatorBalanceMap = HashMap::new();
        let mut all_delegator_pools: DelegatorPoolMap = HashMap::new();
        let mut all_delegator_pool_balances = vec![];
        let mut all_current_delegator_pool_balances = HashMap::new();

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
            let delegator_balances =
                CurrentDelegatorBalance::from_transaction(txn, &mut conn).unwrap();
            all_delegator_balances.extend(delegator_balances);

            // Add delegator pools
            let (delegator_pools, mut delegator_pool_balances, current_delegator_pool_balances) =
                DelegatorPool::from_transaction(txn).unwrap();
            all_delegator_pools.extend(delegator_pools);
            all_delegator_pool_balances.append(&mut delegator_pool_balances);
            all_current_delegator_pool_balances.extend(current_delegator_pool_balances);
        }

        // Getting list of values and sorting by pk in order to avoid postgres deadlock since we're doing multi threaded db writes
        let mut all_current_stake_pool_voters = all_current_stake_pool_voters
            .into_values()
            .collect::<Vec<CurrentStakingPoolVoter>>();
        let mut all_delegator_balances = all_delegator_balances
            .into_values()
            .collect::<Vec<CurrentDelegatorBalance>>();
        let mut all_delegator_pools = all_delegator_pools
            .into_values()
            .collect::<Vec<DelegatorPool>>();
        let mut all_current_delegator_pool_balances = all_current_delegator_pool_balances
            .into_values()
            .collect::<Vec<CurrentDelegatorPoolBalance>>();

        // Sort by PK
        all_current_stake_pool_voters
            .sort_by(|a, b| a.staking_pool_address.cmp(&b.staking_pool_address));
        all_delegator_balances.sort_by(|a, b| {
            (&a.delegator_address, &a.pool_address, &a.pool_type).cmp(&(
                &b.delegator_address,
                &b.pool_address,
                &b.pool_type,
            ))
        });
        all_delegator_pools.sort_by(|a, b| a.staking_pool_address.cmp(&b.staking_pool_address));
        all_current_delegator_pool_balances
            .sort_by(|a, b| a.staking_pool_address.cmp(&b.staking_pool_address));

        let tx_result = insert_to_db(
            &mut conn,
            self.name(),
            start_version,
            end_version,
            all_current_stake_pool_voters,
            all_proposal_votes,
            all_delegator_activities,
            all_delegator_balances,
            all_delegator_pools,
            all_delegator_pool_balances,
            all_current_delegator_pool_balances,
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
