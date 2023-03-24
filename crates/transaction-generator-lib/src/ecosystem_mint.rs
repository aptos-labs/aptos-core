// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0
use crate::{TransactionGenerator, TransactionGeneratorCreator};
use aptos_config::keys::ConfigKey;
use aptos_infallible::RwLock;
use aptos_logger::{sample, sample::SampleRate, warn};
use aptos_sdk::{
    move_types::{language_storage::ModuleId, ident_str},
    transaction_builder::TransactionFactory,
    types::{transaction::{SignedTransaction, EntryFunction, TransactionPayload}, LocalAccount, AccountKey},
};
use async_trait::async_trait;
use rand::{rngs::StdRng, SeedableRng};
use std::{sync::Arc, time::Duration};

use super::TransactionExecutor;


// previewnet:
// private_key: ""
// account : d3d409ec76f1b9efff2afa7e66e2f4d7687d9f6a32eeb02bcd2d2c772b3d1bd9

pub struct EcosystemMintGenerator {
    rng: StdRng,
    txn_factory: TransactionFactory,
    accounts_pool: Arc<RwLock<Vec<LocalAccount>>>,
    module_admin: Arc<LocalAccount>,
    v_improved: bool,
}

impl EcosystemMintGenerator {
    pub fn new(
        rng: StdRng,
        txn_factory: TransactionFactory,
        accounts_pool: Arc<RwLock<Vec<LocalAccount>>>,
        module_admin: Arc<LocalAccount>,
        v_improved: bool,
    ) -> Self {
        Self {
            rng,
            txn_factory,
            accounts_pool,
            module_admin,
            v_improved,
        }
    }
}

#[async_trait]
impl TransactionGenerator for EcosystemMintGenerator {
    fn generate_transactions(
        &mut self,
        accounts: Vec<&mut LocalAccount>,
        transactions_per_account: usize,
    ) -> Vec<SignedTransaction> {
        let mut requests = Vec::with_capacity(accounts.len() * transactions_per_account);

        let unique = if self.v_improved { 2 * transactions_per_account } else { transactions_per_account };

        let needed = accounts.len() * unique;

        let mut accounts_to_burn = get_account_to_burn_from_pool(&self.accounts_pool, needed);
        if accounts_to_burn.is_empty() {
            return Vec::new();
        }

        for fee_payer in accounts {
            for _ in 0..unique {
                let user_account = accounts_to_burn.pop().unwrap();
                requests.push(fee_payer.sign_multi_agent_with_transaction_builder(
                    vec![&self.module_admin, &user_account],
                    self.txn_factory.payload(
                        TransactionPayload::EntryFunction(EntryFunction::new(
                            ModuleId::new(
                                self.module_admin.address(),
                                ident_str!("bugs").to_owned(),
                            ),
                            ident_str!("mint_token").to_owned(),
                            vec![],
                            vec![],
                        ))
                    )
                ));
                if self.v_improved {
                    requests.push(fee_payer.sign_multi_agent_with_transaction_builder(
                        vec![&user_account],
                        self.txn_factory.payload(
                            TransactionPayload::EntryFunction(EntryFunction::new(
                                ModuleId::new(
                                    self.module_admin.address(),
                                    ident_str!("bugs").to_owned(),
                                ),
                                ident_str!("tweak_token").to_owned(),
                                vec![],
                                vec![],
                            ))
                        )
                    ));
                }
            }
        }
        requests
    }
}

pub struct EcosystemMintGeneratorCreator {
    txn_factory: TransactionFactory,
    accounts_pool: Arc<RwLock<Vec<LocalAccount>>>,
    module_admin: Arc<LocalAccount>,
    v_improved: bool,
}

impl EcosystemMintGeneratorCreator {
    pub async fn new(
        txn_factory: TransactionFactory,
        txn_executor: &dyn TransactionExecutor,
        accounts_pool: Arc<RwLock<Vec<LocalAccount>>>,
        v_improved: bool,
    ) -> Self {
        let key = ConfigKey::from_encoded_string(
            match (v_improved, format!("{:?}", txn_factory.get_chain_id()).as_str()) {
                (false, "51") => "0x959d65aac59b6d8f15f05a09c6b61ded2d68621e412ce0d18546c3c06693fa31",
                (false, "testnet") => "0x9b67717bf0ebe7b58feeef68666bf63c4002b8c0707fc34c2ab57c9cf1b41fab",
                (true, "testnet") => "0xda6daa9b185863d559a4e8192044653282671215788366500479b96e9110c61c",
                (true, "51") => "0xda6daa9b185863d559a4e8192044653282671215788366500479b96e9110c61c", // "0xe5ae0d4605c5f4ee61e41a906d440843538cf4ad7e5a5271c60f89ce196c03a9",
                _ => unreachable!(),
            }
        ).unwrap();
        let account_key = AccountKey::from_private_key(key.private_key());
        let address = account_key.authentication_key().derived_address();
        let seq_num = txn_executor.query_sequence_number(address).await.unwrap();

        let module_admin = Arc::new(LocalAccount::new(address, account_key, seq_num));

        Self {
            txn_factory,
            accounts_pool,
            module_admin,
            v_improved,
        }
    }
}

#[async_trait]
impl TransactionGeneratorCreator for EcosystemMintGeneratorCreator {
    async fn create_transaction_generator(&mut self) -> Box<dyn TransactionGenerator> {
        Box::new(EcosystemMintGenerator::new(
            StdRng::from_entropy(),
            self.txn_factory.clone(),
            self.accounts_pool.clone(),
            self.module_admin.clone(),
            self.v_improved,
        ))
    }
}
