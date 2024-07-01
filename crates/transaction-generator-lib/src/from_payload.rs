// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0
use crate::{
    TransactionGenerator, TransactionGeneratorCreator,
};
use aptos_sdk::{
    move_types::{ident_str, language_storage::ModuleId},
    transaction_builder::TransactionFactory,
    types::{transaction::{EntryFunction, SignedTransaction, TransactionPayload}, LocalAccount},
};

pub struct FromPayloadGenerator {
    txn_factory: TransactionFactory,
    payload: TransactionPayload,
}

impl FromPayloadGenerator {
    pub fn new(
        txn_factory: TransactionFactory,
        payload: TransactionPayload,
    ) -> Self {
        Self {
            txn_factory,
            payload,
        }
    }
}

impl TransactionGenerator for FromPayloadGenerator {
    fn generate_transactions(
        &mut self,
        account: &LocalAccount,
        num_to_create: usize,
    ) -> Vec<SignedTransaction> {
        let mut requests = Vec::with_capacity(num_to_create);
        for _ in 0..num_to_create {
            requests.push(
                account.sign_with_transaction_builder(self.txn_factory.payload(
                    self.payload.clone()
                )),
            );
        }

        requests
    }
}

pub struct FromPayloadGeneratorCreator {
    txn_factory: TransactionFactory,
    payload: TransactionPayload,
}

impl FromPayloadGeneratorCreator {
    pub fn new(
        txn_factory: TransactionFactory,
        payload: TransactionPayload,
    ) -> Self {
        Self {
            txn_factory,
            payload,
        }
    }
}

impl TransactionGeneratorCreator for FromPayloadGeneratorCreator {
    fn create_transaction_generator(&self) -> Box<dyn TransactionGenerator> {
        Box::new(FromPayloadGenerator::new(
            self.txn_factory.clone(),
            self.payload.clone(),
        ))
    }
}
