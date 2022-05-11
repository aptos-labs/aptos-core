// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::layout::Layout;
use anyhow::Result;
use aptos_crypto::ed25519::Ed25519PublicKey;
use aptos_global_constants::{
    APTOS_ROOT_KEY, MIN_PRICE_PER_GAS_UNIT, MOVE_MODULES, OPERATOR_KEY, OWNER_KEY,
};
use aptos_management::constants::{self, VALIDATOR_CONFIG, VALIDATOR_OPERATOR};
use aptos_secure_storage::{KVStorage, Namespaced};
use aptos_types::{
    chain_id::ChainId,
    transaction::{
        authenticator::AuthenticationKey, ScriptFunction, Transaction, TransactionPayload,
    },
};
use vm_genesis::Validator;

pub struct GenesisBuilder<S> {
    storage: S,
}

impl<S> GenesisBuilder<S> {
    pub fn new(storage: S) -> Self {
        Self { storage }
    }
}

impl<S: KVStorage> GenesisBuilder<S> {
    fn with_namespace(&self, namespace: &str) -> Namespaced<&S> {
        Namespaced::new(namespace, &self.storage)
    }

    fn with_namespace_mut(&mut self, namespace: &str) -> Namespaced<&mut S> {
        Namespaced::new(namespace, &mut self.storage)
    }

    pub fn set_layout(&mut self, layout: &Layout) -> Result<()> {
        self.with_namespace_mut(constants::COMMON_NS)
            .set(constants::LAYOUT, layout.to_toml()?)
            .map_err(Into::into)
    }

    pub fn layout(&self) -> Result<Layout> {
        let raw_layout = self
            .with_namespace(constants::COMMON_NS)
            .get::<String>(constants::LAYOUT)?
            .value;
        Layout::parse(&raw_layout).map_err(Into::into)
    }

    pub fn set_move_modules(&mut self, modules: Vec<Vec<u8>>) -> Result<()> {
        self.with_namespace_mut(constants::COMMON_NS)
            .set(MOVE_MODULES, modules)
            .map_err(Into::into)
    }

    pub fn move_modules(&self) -> Result<Vec<Vec<u8>>> {
        self.with_namespace(constants::COMMON_NS)
            .get(MOVE_MODULES)
            .map(|r| r.value)
            .map_err(Into::into)
    }

    pub fn set_root_key(&mut self, root_key: Ed25519PublicKey) -> Result<()> {
        let layout = self.layout()?;
        self.with_namespace_mut(&layout.aptos_root)
            .set(APTOS_ROOT_KEY, root_key)
            .map_err(Into::into)
    }

    pub fn root_key(&self) -> Result<Ed25519PublicKey> {
        let layout = self.layout()?;
        self.with_namespace(&layout.aptos_root)
            .get(APTOS_ROOT_KEY)
            .map(|r| r.value)
            .map_err(Into::into)
    }

    pub fn set_operator_key(
        &mut self,
        operator_namespace: &str,
        operator_key: Ed25519PublicKey,
    ) -> Result<()> {
        self.with_namespace_mut(operator_namespace)
            .set(OPERATOR_KEY, operator_key)
            .map_err(Into::into)
    }

    pub fn operator_key(&self, operator: &str) -> Result<Ed25519PublicKey> {
        self.with_namespace(operator)
            .get(OPERATOR_KEY)
            .map(|r| r.value)
            .map_err(Into::into)
    }

    pub fn set_operator(&mut self, validator: &str, operator: &str) -> Result<()> {
        self.with_namespace_mut(validator)
            .set(VALIDATOR_OPERATOR, operator)
            .map_err(Into::into)
    }

    pub fn operator(&self, validator: &str) -> Result<String> {
        self.with_namespace(validator)
            .get(VALIDATOR_OPERATOR)
            .map(|r| r.value)
            .map_err(Into::into)
    }

    pub fn validators(&self) -> Result<Vec<Validator>> {
        let layout = self.layout()?;
        let mut validators = Vec::new();
        for owner in &layout.owners {
            let name = owner.as_bytes().to_vec();
            let address = aptos_config::utils::default_validator_owner_auth_key_from_name(&name)
                .derived_address();
            let auth_key = self
                .owner_key(owner)
                .map_or(AuthenticationKey::zero(), |k| {
                    AuthenticationKey::ed25519(&k)
                });
            let operator = self.operator(owner)?;
            let operator_auth_key = AuthenticationKey::ed25519(&self.operator_key(&operator)?);
            let operator_address = operator_auth_key.derived_address();
            let validator_config = self.validator_config(&operator)?;
            let consensus_pubkey = bcs::from_bytes(&validator_config.args()[1])?;
            let network_address = bcs::from_bytes(&validator_config.args()[2])?;
            let full_node_network_address = bcs::from_bytes(&validator_config.args()[3])?;
            validators.push(Validator {
                address,
                auth_key,
                consensus_pubkey,
                operator_address,
                operator_auth_key,
                network_address,
                full_node_network_address,
                stake_amount: 1,
            })
        }
        Ok(validators)
    }

    pub fn set_owner_key(
        &mut self,
        owner_namespace: &str,
        owner_key: Ed25519PublicKey,
    ) -> Result<()> {
        self.with_namespace_mut(owner_namespace)
            .set(OWNER_KEY, owner_key)
            .map_err(Into::into)
    }

    pub fn owner_key(&self, owner: &str) -> Result<Ed25519PublicKey> {
        self.with_namespace(owner)
            .get(OWNER_KEY)
            .map(|r| r.value)
            .map_err(Into::into)
    }

    pub fn set_validator_config(
        &mut self,
        operator: &str,
        validator_config_transaction: &Transaction,
    ) -> Result<()> {
        self.with_namespace_mut(operator)
            .set(VALIDATOR_CONFIG, validator_config_transaction)
            .map_err(Into::into)
    }

    pub fn validator_config(&self, operator: &str) -> Result<ScriptFunction> {
        let txn = self
            .with_namespace(operator)
            .get::<Transaction>(VALIDATOR_CONFIG)
            .map(|r| r.value)?;
        if let Transaction::UserTransaction(txn) = txn {
            Some(txn)
        } else {
            None
        }
        .and_then(|txn| {
            if let TransactionPayload::ScriptFunction(txn) =
                txn.into_raw_transaction().into_payload()
            {
                Some(txn)
            } else {
                None
            }
        })
        .ok_or_else(|| anyhow::anyhow!("Invalid Validator Config"))
    }

    pub fn set_min_price_per_gas_unit(&mut self, min_price_per_gas_unit: u64) -> Result<()> {
        self.with_namespace_mut(constants::COMMON_NS)
            .set(MIN_PRICE_PER_GAS_UNIT, min_price_per_gas_unit)
            .map_err(Into::into)
    }

    pub fn min_price_per_gas_unit(&self) -> Result<u64> {
        self.with_namespace(constants::COMMON_NS)
            .get::<u64>(MIN_PRICE_PER_GAS_UNIT)
            .map(|r| r.value)
            .map_err(Into::into)
    }

    pub fn build(&self, chain_id: ChainId) -> Result<Transaction> {
        let aptos_root_key = self.root_key()?;
        let validators = self.validators()?;
        let move_modules = self.move_modules()?;
        let min_price_per_gas_unit = self.min_price_per_gas_unit().unwrap_or(1);

        let genesis = vm_genesis::encode_genesis_transaction(
            aptos_root_key,
            &validators,
            &move_modules,
            chain_id,
            min_price_per_gas_unit,
        );

        Ok(genesis)
    }
}
