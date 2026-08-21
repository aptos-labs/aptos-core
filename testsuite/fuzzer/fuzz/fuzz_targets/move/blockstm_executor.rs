// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use aptos_keygen::KeyGen;
use aptos_transaction_simulation::{
    Account, AccountData, InMemoryStateStore, SimulationStateStore,
};
use aptos_types::{
    account_address::AccountAddress,
    account_config::{CoinInfoResource, ConcurrentSupplyResource, ObjectGroupResource},
    chain_id::ChainId,
    on_chain_config::FeatureFlag,
    state_store::{state_key::StateKey, TStateView},
    transaction::{TransactionOutput, TransactionStatus},
    write_set::{WriteOp, WriteSet, WriteSetMut},
    AptosCoinType, CoinType,
};
use move_core_types::move_resource::MoveStructType;
use std::collections::BTreeSet;

const RNG_SEED: [u8; 32] = [9u8; 32];
const DEFAULT_ACCOUNT_FUND_AMOUNT: u64 = 1_000_000_000_000_000;

pub(crate) struct BlockFuzzExecutor {
    state_store: InMemoryStateStore,
    rng: KeyGen,
}

impl BlockFuzzExecutor {
    pub(crate) fn from_genesis(write_set: &WriteSet, chain_id: ChainId) -> Self {
        let state_store = InMemoryStateStore::new();
        state_store.set_chain_id(chain_id).unwrap();
        state_store.apply_write_set(write_set).unwrap();
        Self {
            state_store,
            rng: KeyGen::from_seed(RNG_SEED),
        }
    }

    pub(crate) fn state_store(&self) -> &InMemoryStateStore {
        &self.state_store
    }

    pub(crate) fn get_state_view(&self) -> &InMemoryStateStore {
        &self.state_store
    }

    pub(crate) fn apply_transaction_outputs(&self, outputs: &[TransactionOutput]) {
        for output in outputs {
            if matches!(output.status(), TransactionStatus::Keep(_)) {
                self.state_store
                    .apply_write_set(output.write_set())
                    .unwrap();
            }
        }
    }

    pub(crate) fn create_accounts(
        &mut self,
        size: usize,
        balance: u64,
        sequence_number: u64,
    ) -> Vec<Account> {
        (0..size)
            .map(|_| {
                let account_data =
                    AccountData::new_from_seed(&mut self.rng, balance, sequence_number);
                self.add_account_data(&account_data);
                account_data.into_account()
            })
            .collect()
    }

    pub(crate) fn new_unfunded_account(&mut self) -> Account {
        Account::new_from_seed(&mut self.rng)
    }

    pub(crate) fn new_account_at(&mut self, address: AccountAddress) -> Account {
        let account = Account::new_genesis_account(address);
        let features = self.state_store.get_features().unwrap_or_default();
        let account_data = AccountData::with_account(
            account,
            DEFAULT_ACCOUNT_FUND_AMOUNT,
            0,
            features.is_enabled(FeatureFlag::NEW_ACCOUNTS_DEFAULT_TO_FA_APT_STORE),
            features.is_enabled(FeatureFlag::DEFAULT_TO_CONCURRENT_FUNGIBLE_BALANCE),
        );
        self.add_account_data(&account_data);
        account_data.into_account()
    }

    fn add_account_data(&self, account_data: &AccountData) {
        self.state_store.add_account_data(account_data).unwrap();

        if let Some(new_supply) = account_data.coin_balance()
            && new_supply != 0
        {
            let coin_info = self.read_apt_coin_info_resource();
            let old_supply = self.read_coin_supply(&coin_info);
            self.state_store
                .apply_write_set(
                    &coin_info
                        .to_writeset(old_supply + new_supply as u128)
                        .unwrap(),
                )
                .unwrap();
        }

        if let Some(new_supply) = account_data.fungible_balance()
            && new_supply != 0
        {
            let mut group = self.read_fa_supply_resource_group();
            let mut supply = bcs::from_bytes::<ConcurrentSupplyResource>(
                group
                    .group
                    .get(&ConcurrentSupplyResource::struct_tag())
                    .unwrap(),
            )
            .unwrap();
            supply
                .current
                .set(supply.current.get() + new_supply as u128);
            group
                .group
                .insert(
                    ConcurrentSupplyResource::struct_tag(),
                    bcs::to_bytes(&supply).unwrap(),
                )
                .unwrap();
            self.state_store
                .apply_write_set(
                    &WriteSetMut::new(vec![(
                        StateKey::resource_group(
                            &AccountAddress::TEN,
                            &ObjectGroupResource::struct_tag(),
                        ),
                        WriteOp::legacy_modification(bcs::to_bytes(&group).unwrap().into()),
                    )])
                    .freeze()
                    .unwrap(),
                )
                .unwrap();
        }
    }

    fn read_apt_coin_info_resource(&self) -> CoinInfoResource<AptosCoinType> {
        self.state_store
            .get_resource(AptosCoinType::coin_info_address())
            .unwrap()
            .expect("APT coin info must exist in genesis")
    }

    fn read_coin_supply(&self, coin_info: &CoinInfoResource<AptosCoinType>) -> u128 {
        self.state_store
            .get_state_value_bytes(&coin_info.supply_aggregator_state_key())
            .unwrap()
            .map(|bytes| bcs::from_bytes::<u128>(&bytes).unwrap())
            .unwrap_or_default()
    }

    fn read_fa_supply_resource_group(&self) -> ObjectGroupResource {
        let bytes = self
            .state_store
            .get_state_value_bytes(&StateKey::resource_group(
                &AccountAddress::TEN,
                &ObjectGroupResource::struct_tag(),
            ))
            .unwrap()
            .expect("FA supply resource group must exist in genesis");
        bcs::from_bytes(&bytes).unwrap()
    }
}

pub(crate) fn assert_outputs_equal(
    first: &[TransactionOutput],
    first_name: &str,
    second: &[TransactionOutput],
    second_name: &str,
) {
    assert_eq!(
        first.len(),
        second.len(),
        "Transaction outputs size mismatch: in {:?} and in {:?}",
        first_name,
        second_name,
    );

    for (idx, (first_output, second_output)) in first.iter().zip(second.iter()).enumerate() {
        assert_eq!(
            first_output.status(),
            second_output.status(),
            "Different statuses for {:?} and {:?} for transaction outputs at index {}",
            first_name,
            second_name,
            idx,
        );

        assert_eq!(
            first_output.try_extract_fee_statement().unwrap_or_default(),
            second_output
                .try_extract_fee_statement()
                .unwrap_or_default(),
            "Different gas used for {:?} and {:?} for transaction outputs at index {}",
            first_name,
            second_name,
            idx,
        );

        let keys = first_output
            .write_set()
            .write_op_iter()
            .chain(second_output.write_set().write_op_iter())
            .map(|(key, _)| key)
            .collect::<BTreeSet<_>>();
        let mut differences = vec![];
        for key in keys {
            let write1 = first_output.write_set().get_write_op(key);
            let write2 = second_output.write_set().get_write_op(key);
            if write1 != write2 {
                differences.push(format!(
                    "Write for {:?} differs: {:?} vs {:?}",
                    key, write1, write2
                ));
            }
        }
        if !differences.is_empty() {
            println!("Differences:\n{}", differences.join("\n"));
        }
        assert!(
            differences.is_empty(),
            "First write op mismatch for transaction output at index {}, between {} and {}",
            idx,
            first_name,
            second_name,
        );

        assert_eq!(
            first_output, second_output,
            "first transaction output mismatch at index {}, for {} and {}",
            idx, first_name, second_name,
        );
    }
}
