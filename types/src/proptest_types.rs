// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    access_path::AccessPath,
    account_address::{self, AccountAddress},
    account_config::{AccountResource, CoinStoreResource},
    account_state::AccountState,
    aggregate_signature::PartialSignatures,
    block_info::{BlockInfo, Round},
    block_metadata::BlockMetadata,
    chain_id::ChainId,
    contract_event::ContractEvent,
    epoch_state::EpochState,
    event::{EventHandle, EventKey},
    ledger_info::{generate_ledger_info_with_sig, LedgerInfo, LedgerInfoWithSignatures},
    on_chain_config::ValidatorSet,
    proof::TransactionInfoListWithProof,
    state_store::{state_key::StateKey, state_value::StateValue},
    transaction::{
        ChangeSet, ExecutionStatus, Module, ModuleBundle, RawTransaction, Script,
        SignatureCheckedTransaction, SignedTransaction, Transaction, TransactionArgument,
        TransactionInfo, TransactionListWithProof, TransactionPayload, TransactionStatus,
        TransactionToCommit, Version, WriteSetPayload,
    },
    validator_info::ValidatorInfo,
    validator_signer::ValidatorSigner,
    validator_verifier::{ValidatorConsensusInfo, ValidatorVerifier},
    vm_status::VMStatus,
    write_set::{WriteOp, WriteSet, WriteSetMut},
};
use aptos_crypto::{
    bls12381::{self, bls12381_keys},
    ed25519::{self, Ed25519PrivateKey, Ed25519PublicKey},
    test_utils::KeyPair,
    traits::*,
    HashValue,
};
use move_deps::move_core_types::language_storage::TypeTag;
use proptest::{
    collection::{vec, SizeRange},
    option,
    prelude::*,
    sample::Index,
};
use proptest_derive::Arbitrary;
use serde_json::Value;
use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    convert::TryFrom,
    iter::Iterator,
};

impl WriteOp {
    pub fn value_strategy() -> impl Strategy<Value = Self> {
        vec(any::<u8>(), 0..64).prop_map(WriteOp::Modification)
    }

    pub fn deletion_strategy() -> impl Strategy<Value = Self> {
        Just(WriteOp::Deletion)
    }
}

impl Arbitrary for WriteOp {
    type Parameters = ();
    fn arbitrary_with(_args: ()) -> Self::Strategy {
        prop_oneof![Self::deletion_strategy(), Self::value_strategy()].boxed()
    }

    type Strategy = BoxedStrategy<Self>;
}

impl Arbitrary for WriteSetPayload {
    type Parameters = ();
    fn arbitrary_with(_args: ()) -> Self::Strategy {
        any::<ChangeSet>().prop_map(WriteSetPayload::Direct).boxed()
    }

    type Strategy = BoxedStrategy<Self>;
}

impl Arbitrary for WriteSet {
    type Parameters = ();
    fn arbitrary_with(_args: ()) -> Self::Strategy {
        // XXX there's no checking for repeated access paths here, nor in write_set. Is that
        // important? Not sure.
        vec((any::<AccessPath>(), any::<WriteOp>()), 0..64)
            .prop_map(|write_set| {
                let write_set_mut =
                    WriteSetMut::new(write_set.iter().map(|(access_path, write_op)| {
                        (StateKey::AccessPath(access_path.clone()), write_op.clone())
                    }));
                write_set_mut
                    .freeze()
                    .expect("generated write sets should always be valid")
            })
            .boxed()
    }

    type Strategy = BoxedStrategy<Self>;
}

impl Arbitrary for ChangeSet {
    type Parameters = ();
    fn arbitrary_with(_args: ()) -> Self::Strategy {
        (any::<WriteSet>(), vec(any::<ContractEvent>(), 0..10))
            .prop_map(|(ws, events)| ChangeSet::new(ws, events))
            .boxed()
    }

    type Strategy = BoxedStrategy<Self>;
}

impl EventKey {
    pub fn strategy_impl(
        account_address_strategy: impl Strategy<Value = AccountAddress>,
    ) -> impl Strategy<Value = Self> {
        // We only generate small counters so that it won't overflow.
        (account_address_strategy, 0..std::u64::MAX / 2)
            .prop_map(|(account_address, counter)| EventKey::new(counter, account_address))
    }
}

impl Arbitrary for EventKey {
    type Parameters = ();
    type Strategy = BoxedStrategy<Self>;

    fn arbitrary_with(_args: Self::Parameters) -> Self::Strategy {
        EventKey::strategy_impl(any::<AccountAddress>()).boxed()
    }
}

#[derive(Debug)]
struct AccountInfo {
    address: AccountAddress,
    private_key: Ed25519PrivateKey,
    public_key: Ed25519PublicKey,
    consensus_private_key: bls12381::PrivateKey,
    sequence_number: u64,
    sent_event_handle: EventHandle,
    received_event_handle: EventHandle,
}

impl AccountInfo {
    pub fn new(
        private_key: Ed25519PrivateKey,
        consensus_private_key: bls12381::PrivateKey,
    ) -> Self {
        let public_key = private_key.public_key();
        let address = account_address::from_public_key(&public_key);
        Self {
            address,
            private_key,
            public_key,
            consensus_private_key,
            sequence_number: 0,
            sent_event_handle: EventHandle::new(EventKey::new(0, address), 0),
            received_event_handle: EventHandle::new(EventKey::new(1, address), 0),
        }
    }
}

#[derive(Debug)]
pub struct AccountInfoUniverse {
    accounts: Vec<AccountInfo>,
    epoch: u64,
    round: Round,
    next_version: Version,
    validator_set_by_epoch: BTreeMap<u64, Vec<ValidatorSigner>>,
}

impl AccountInfoUniverse {
    fn new(
        account_private_keys: Vec<Ed25519PrivateKey>,
        consensus_private_keys: Vec<bls12381::PrivateKey>,
        epoch: u64,
        round: Round,
        next_version: Version,
    ) -> Self {
        let mut accounts: Vec<_> = account_private_keys
            .into_iter()
            .zip(consensus_private_keys.into_iter())
            .map(|(private_key, consensus_private_key)| {
                AccountInfo::new(private_key, consensus_private_key)
            })
            .collect();
        accounts.sort_by(|a, b| a.address.cmp(&b.address));
        let validator_signer = ValidatorSigner::new(
            accounts[0].address,
            accounts[0].consensus_private_key.clone(),
        );
        let validator_set_by_epoch = vec![(0, vec![validator_signer])].into_iter().collect();

        Self {
            accounts,
            epoch,
            round,
            next_version,
            validator_set_by_epoch,
        }
    }

    fn get_account_info(&self, account_index: Index) -> &AccountInfo {
        account_index.get(&self.accounts)
    }

    fn get_account_infos_dedup(&self, account_indices: &[Index]) -> Vec<&AccountInfo> {
        account_indices
            .iter()
            .map(|idx| idx.index(self.accounts.len()))
            .collect::<BTreeSet<_>>()
            .iter()
            .map(|idx| &self.accounts[*idx])
            .collect()
    }

    fn get_account_info_mut(&mut self, account_index: Index) -> &mut AccountInfo {
        account_index.get_mut(self.accounts.as_mut_slice())
    }

    fn get_and_bump_round(&mut self) -> Round {
        let round = self.round;
        self.round += 1;
        round
    }

    fn bump_and_get_version(&mut self, block_size: usize) -> Version {
        self.next_version += block_size as u64;
        self.next_version - 1
    }

    fn get_epoch(&self) -> u64 {
        self.epoch
    }

    fn get_and_bump_epoch(&mut self) -> u64 {
        let epoch = self.epoch;
        self.epoch += 1;
        epoch
    }

    pub fn get_validator_set(&self, epoch: u64) -> &[ValidatorSigner] {
        &self.validator_set_by_epoch[&epoch]
    }

    fn set_validator_set(&mut self, epoch: u64, validator_set: Vec<ValidatorSigner>) {
        self.validator_set_by_epoch.insert(epoch, validator_set);
    }
}

impl Arbitrary for AccountInfoUniverse {
    type Parameters = usize;
    fn arbitrary_with(num_accounts: Self::Parameters) -> Self::Strategy {
        vec(
            (
                ed25519::keypair_strategy(),
                bls12381_keys::keypair_strategy(),
            ),
            num_accounts,
        )
        .prop_map(|kps| {
            let mut account_private_keys = vec![];
            let mut consensus_private_keys = vec![];
            for (kp1, kp2) in kps {
                account_private_keys.push(kp1.private_key);
                consensus_private_keys.push(kp2.private_key);
            }
            AccountInfoUniverse::new(
                account_private_keys,
                consensus_private_keys,
                /* epoch = */ 0,
                /* round = */ 0,
                /* next_version = */ 0,
            )
        })
        .boxed()
    }

    fn arbitrary() -> Self::Strategy {
        unimplemented!("Size of the universe must be provided explicitly (use any_with instead).")
    }

    type Strategy = BoxedStrategy<Self>;
}

#[derive(Arbitrary, Debug)]
pub struct RawTransactionGen {
    payload: TransactionPayload,
    max_gas_amount: u64,
    gas_unit_price: u64,
    expiration_time_secs: u64,
}

impl RawTransactionGen {
    pub fn materialize(
        self,
        sender_index: Index,
        universe: &mut AccountInfoUniverse,
    ) -> RawTransaction {
        let mut sender_info = universe.get_account_info_mut(sender_index);

        let sequence_number = sender_info.sequence_number;
        sender_info.sequence_number += 1;

        new_raw_transaction(
            sender_info.address,
            sequence_number,
            self.payload,
            self.max_gas_amount,
            self.gas_unit_price,
            self.expiration_time_secs,
        )
    }
}

impl RawTransaction {
    fn strategy_impl(
        address_strategy: impl Strategy<Value = AccountAddress>,
        payload_strategy: impl Strategy<Value = TransactionPayload>,
    ) -> impl Strategy<Value = Self> {
        // XXX what other constraints do these need to obey?
        (
            address_strategy,
            any::<u64>(),
            payload_strategy,
            any::<u64>(),
            any::<u64>(),
            any::<u64>(),
        )
            .prop_map(
                |(
                    sender,
                    sequence_number,
                    payload,
                    max_gas_amount,
                    gas_unit_price,
                    expiration_time_secs,
                )| {
                    new_raw_transaction(
                        sender,
                        sequence_number,
                        payload,
                        max_gas_amount,
                        gas_unit_price,
                        expiration_time_secs,
                    )
                },
            )
    }
}

fn new_raw_transaction(
    sender: AccountAddress,
    sequence_number: u64,
    payload: TransactionPayload,
    max_gas_amount: u64,
    gas_unit_price: u64,
    expiration_time_secs: u64,
) -> RawTransaction {
    let chain_id = ChainId::test();
    match payload {
        TransactionPayload::ModuleBundle(module) => RawTransaction::new_module_bundle(
            sender,
            sequence_number,
            module,
            max_gas_amount,
            gas_unit_price,
            expiration_time_secs,
            chain_id,
        ),
        TransactionPayload::Script(script) => RawTransaction::new_script(
            sender,
            sequence_number,
            script,
            max_gas_amount,
            gas_unit_price,
            expiration_time_secs,
            chain_id,
        ),
        TransactionPayload::EntryFunction(script_fn) => RawTransaction::new_entry_function(
            sender,
            sequence_number,
            script_fn,
            max_gas_amount,
            gas_unit_price,
            expiration_time_secs,
            chain_id,
        ),
    }
}

impl Arbitrary for RawTransaction {
    type Parameters = ();
    fn arbitrary_with(_args: ()) -> Self::Strategy {
        Self::strategy_impl(any::<AccountAddress>(), any::<TransactionPayload>()).boxed()
    }

    type Strategy = BoxedStrategy<Self>;
}

impl SignatureCheckedTransaction {
    // This isn't an Arbitrary impl because this doesn't generate *any* possible SignedTransaction,
    // just one kind of them.
    pub fn script_strategy(
        keypair_strategy: impl Strategy<Value = KeyPair<Ed25519PrivateKey, Ed25519PublicKey>>,
    ) -> impl Strategy<Value = Self> {
        Self::strategy_impl(keypair_strategy, TransactionPayload::script_strategy())
    }

    pub fn module_strategy(
        keypair_strategy: impl Strategy<Value = KeyPair<Ed25519PrivateKey, Ed25519PublicKey>>,
    ) -> impl Strategy<Value = Self> {
        Self::strategy_impl(keypair_strategy, TransactionPayload::module_strategy())
    }

    fn strategy_impl(
        keypair_strategy: impl Strategy<Value = KeyPair<Ed25519PrivateKey, Ed25519PublicKey>>,
        payload_strategy: impl Strategy<Value = TransactionPayload>,
    ) -> impl Strategy<Value = Self> {
        (keypair_strategy, payload_strategy)
            .prop_flat_map(|(keypair, payload)| {
                let address = account_address::from_public_key(&keypair.public_key);
                (
                    Just(keypair),
                    RawTransaction::strategy_impl(Just(address), Just(payload)),
                )
            })
            .prop_flat_map(|(keypair, raw_txn)| {
                prop_oneof![
                    Just(
                        raw_txn
                            .clone()
                            .sign(&keypair.private_key, keypair.public_key.clone())
                            .expect("signing should always work")
                    ),
                    Just(
                        raw_txn
                            .multi_sign_for_testing(&keypair.private_key, keypair.public_key)
                            .expect("signing should always work")
                    ),
                ]
            })
    }
}

#[derive(Arbitrary, Debug)]
pub struct SignatureCheckedTransactionGen {
    raw_transaction_gen: RawTransactionGen,
}

impl SignatureCheckedTransactionGen {
    pub fn materialize(
        self,
        sender_index: Index,
        universe: &mut AccountInfoUniverse,
    ) -> SignatureCheckedTransaction {
        let raw_txn = self.raw_transaction_gen.materialize(sender_index, universe);
        let account_info = universe.get_account_info(sender_index);
        raw_txn
            .sign(&account_info.private_key, account_info.public_key.clone())
            .expect("Signing raw transaction should work.")
    }
}

impl Arbitrary for SignatureCheckedTransaction {
    type Parameters = ();
    fn arbitrary_with(_args: ()) -> Self::Strategy {
        Self::strategy_impl(ed25519::keypair_strategy(), any::<TransactionPayload>()).boxed()
    }

    type Strategy = BoxedStrategy<Self>;
}

/// This `Arbitrary` impl only generates valid signed transactions. TODO: maybe add invalid ones?
impl Arbitrary for SignedTransaction {
    type Parameters = ();
    fn arbitrary_with(_args: ()) -> Self::Strategy {
        any::<SignatureCheckedTransaction>()
            .prop_map(|txn| txn.into_inner())
            .boxed()
    }

    type Strategy = BoxedStrategy<Self>;
}

impl TransactionPayload {
    pub fn script_strategy() -> impl Strategy<Value = Self> {
        any::<Script>().prop_map(TransactionPayload::Script)
    }

    pub fn module_strategy() -> impl Strategy<Value = Self> {
        any::<Module>()
            .prop_map(|module| TransactionPayload::ModuleBundle(ModuleBundle::from(module)))
    }
}

prop_compose! {
    fn arb_transaction_status()(vm_status in any::<VMStatus>()) -> TransactionStatus {
        vm_status.into()
    }
}

prop_compose! {
    fn arb_pubkey()(keypair in ed25519::keypair_strategy()) -> AccountAddress {
            account_address::from_public_key(&keypair.public_key)
    }
}

impl Arbitrary for TransactionStatus {
    type Parameters = ();
    fn arbitrary_with(_args: Self::Parameters) -> Self::Strategy {
        arb_transaction_status().boxed()
    }

    type Strategy = BoxedStrategy<Self>;
}

impl Arbitrary for TransactionPayload {
    type Parameters = ();
    fn arbitrary_with(_args: ()) -> Self::Strategy {
        // Most transactions in practice will be programs, but other parts of the system should
        // at least not choke on write set strategies so introduce them with decent probability.
        // The figures below are probability weights.
        prop_oneof![
            4 => Self::script_strategy(),
            1 => Self::module_strategy(),
        ]
        .boxed()
    }

    type Strategy = BoxedStrategy<Self>;
}

impl Arbitrary for Script {
    type Parameters = ();
    type Strategy = BoxedStrategy<Self>;

    fn arbitrary_with(_args: ()) -> Self::Strategy {
        // XXX This should eventually be an actually valid program, maybe?
        // The vector sizes are picked out of thin air.
        (
            vec(any::<u8>(), 0..100),
            vec(any::<TypeTag>(), 0..4),
            vec(any::<TransactionArgument>(), 0..10),
        )
            .prop_map(|(code, ty_args, args)| Script::new(code, ty_args, args))
            .boxed()
    }
}

impl Arbitrary for Module {
    type Parameters = ();
    type Strategy = BoxedStrategy<Self>;

    fn arbitrary_with(_args: ()) -> Self::Strategy {
        // XXX How should we generate random modules?
        // The vector sizes are picked out of thin air.
        vec(any::<u8>(), 0..100).prop_map(Module::new).boxed()
    }
}

prop_compose! {
    fn arb_validator_for_ledger_info(ledger_info: LedgerInfo)(
        ledger_info in Just(ledger_info),
        account_keypair in ed25519::keypair_strategy(),
        consensus_keypair in bls12381_keys::keypair_strategy(),
    ) -> (AccountAddress, ValidatorConsensusInfo,  bls12381::Signature) {
        let signature = consensus_keypair.private_key.sign(&ledger_info).unwrap();
        let address = account_address::from_public_key(&account_keypair.public_key);
        (address, ValidatorConsensusInfo::new(address, consensus_keypair.public_key, 1), signature)
    }
}

impl Arbitrary for LedgerInfoWithSignatures {
    type Parameters = ();
    fn arbitrary_with(_args: Self::Parameters) -> Self::Strategy {
        (any::<LedgerInfo>(), (1usize..100))
            .prop_flat_map(|(ledger_info, num_validators_range)| {
                (
                    Just(ledger_info.clone()),
                    vec(
                        arb_validator_for_ledger_info(ledger_info),
                        num_validators_range,
                    ),
                )
            })
            .prop_map(|(ledger_info, validator_infos)| {
                let validator_verifier = ValidatorVerifier::new_with_quorum_voting_power(
                    validator_infos.iter().map(|x| x.1.clone()).collect(),
                    validator_infos.len() as u128 / 2,
                )
                .unwrap();
                let partial_sig = PartialSignatures::new(
                    validator_infos.iter().map(|x| (x.0, x.2.clone())).collect(),
                );
                LedgerInfoWithSignatures::new(
                    ledger_info,
                    validator_verifier
                        .aggregate_signatures(&partial_sig)
                        .unwrap(),
                )
            })
            .boxed()
    }

    type Strategy = BoxedStrategy<Self>;
}

#[derive(Arbitrary, Debug)]
pub struct ContractEventGen {
    type_tag: TypeTag,
    payload: Vec<u8>,
    use_sent_key: bool,
}

impl ContractEventGen {
    pub fn materialize(
        self,
        account_index: Index,
        universe: &mut AccountInfoUniverse,
    ) -> ContractEvent {
        let account_info = universe.get_account_info_mut(account_index);
        let event_handle = if self.use_sent_key {
            &mut account_info.sent_event_handle
        } else {
            &mut account_info.received_event_handle
        };
        let sequence_number = event_handle.count();
        *event_handle.count_mut() += 1;
        let event_key = event_handle.key();

        ContractEvent::new(*event_key, sequence_number, self.type_tag, self.payload)
    }
}

#[derive(Arbitrary, Debug)]
pub struct AccountResourceGen;

impl AccountResourceGen {
    pub fn materialize(
        self,
        account_index: Index,
        universe: &AccountInfoUniverse,
    ) -> AccountResource {
        let account_info = universe.get_account_info(account_index);
        AccountResource::new(
            account_info.sequence_number,
            account_info.public_key.to_bytes().to_vec(),
            EventHandle::random(0),
            EventHandle::random(1),
        )
    }
}

#[derive(Arbitrary, Debug)]
pub struct CoinStoreResourceGen {
    coin: u64,
}

impl CoinStoreResourceGen {
    pub fn materialize(self) -> CoinStoreResource {
        CoinStoreResource::new(
            self.coin,
            false,
            EventHandle::random(0),
            EventHandle::random(0),
        )
    }
}

#[derive(Arbitrary, Debug)]
pub struct AccountStateGen {
    balance_resource_gen: CoinStoreResourceGen,
    account_resource_gen: AccountResourceGen,
}

impl AccountStateGen {
    pub fn materialize(self, account_index: Index, universe: &AccountInfoUniverse) -> AccountState {
        let address = universe.get_account_info(account_index).address;
        let account_resource = self
            .account_resource_gen
            .materialize(account_index, universe);
        let balance_resource = self.balance_resource_gen.materialize();
        AccountState::try_from((address, &account_resource, &balance_resource)).unwrap()
    }
}

impl EventHandle {
    pub fn strategy_impl(
        event_key_strategy: impl Strategy<Value = EventKey>,
    ) -> impl Strategy<Value = Self> {
        // We only generate small counters so that it won't overflow.
        (event_key_strategy, 0..std::u64::MAX / 2)
            .prop_map(|(event_key, counter)| EventHandle::new(event_key, counter))
    }
}

impl Arbitrary for EventHandle {
    type Parameters = ();
    type Strategy = BoxedStrategy<Self>;

    fn arbitrary_with(_args: Self::Parameters) -> Self::Strategy {
        EventHandle::strategy_impl(any::<EventKey>()).boxed()
    }
}

impl ContractEvent {
    pub fn strategy_impl(
        event_key_strategy: impl Strategy<Value = EventKey>,
    ) -> impl Strategy<Value = Self> {
        (
            event_key_strategy,
            any::<u64>(),
            any::<TypeTag>(),
            vec(any::<u8>(), 1..10),
        )
            .prop_map(|(event_key, seq_num, type_tag, event_data)| {
                ContractEvent::new(event_key, seq_num, type_tag, event_data)
            })
    }
}

impl Arbitrary for ContractEvent {
    type Parameters = ();
    fn arbitrary_with(_args: Self::Parameters) -> Self::Strategy {
        ContractEvent::strategy_impl(any::<EventKey>()).boxed()
    }

    type Strategy = BoxedStrategy<Self>;
}

impl Arbitrary for TransactionToCommit {
    type Parameters = ();
    fn arbitrary_with(_args: Self::Parameters) -> Self::Strategy {
        (
            any_with::<AccountInfoUniverse>(1),
            any::<TransactionToCommitGen>(),
        )
            .prop_map(|(mut universe, gen)| gen.materialize(&mut universe))
            .boxed()
    }

    type Strategy = BoxedStrategy<Self>;
}

/// Represents information already determined for generating a `TransactionToCommit`, along with
/// to be determined information that needs to settle upon `materialize()`, for example a to be
/// determined account can be represented by an `Index` which will be materialized to an entry in
/// the `AccountInfoUniverse`.
///
/// See `TransactionToCommitGen::materialize()` and supporting types.
#[derive(Debug)]
pub struct TransactionToCommitGen {
    /// Transaction sender and the transaction itself.
    transaction_gen: (Index, SignatureCheckedTransactionGen),
    /// Events: account and event content.
    event_gens: Vec<(Index, ContractEventGen)>,
    /// State updates: account and the blob.
    /// N.B. the transaction sender and event owners must be updated to reflect information such as
    /// sequence numbers so that test data generated through this is more realistic and logical.
    account_state_gens: Vec<(Index, AccountStateGen)>,
    /// Gas used.
    gas_used: u64,
    /// Transaction status
    status: ExecutionStatus,
}

impl TransactionToCommitGen {
    /// Materialize considering current states in the universe.
    pub fn materialize(self, universe: &mut AccountInfoUniverse) -> TransactionToCommit {
        let (sender_index, txn_gen) = self.transaction_gen;
        let transaction = txn_gen.materialize(sender_index, universe).into_inner();

        let events = self
            .event_gens
            .into_iter()
            .map(|(index, event_gen)| event_gen.materialize(index, universe))
            .collect();

        let (state_updates, write_set): (HashMap<_, _>, BTreeMap<_, _>) = self
            .account_state_gens
            .into_iter()
            .flat_map(|(index, account_gen)| {
                let address = universe.get_account_info(index).address;
                account_gen
                    .materialize(index, universe)
                    .into_resource_iter()
                    .map(move |(key, value)| {
                        let state_key = StateKey::AccessPath(AccessPath::new(address, key));
                        (
                            (state_key.clone(), Some(StateValue::from(value.clone()))),
                            (state_key, WriteOp::Modification(value)),
                        )
                    })
            })
            .unzip();
        TransactionToCommit::new(
            Transaction::UserTransaction(transaction),
            TransactionInfo::new_placeholder(self.gas_used, None, self.status),
            state_updates,
            WriteSetMut::new(write_set).freeze().expect("Cannot fail"),
            events,
            false, /* event_gen never generates reconfig events */
        )
    }

    pub fn take_account_gens(&mut self) -> Vec<(Index, AccountStateGen)> {
        let mut ret = Vec::new();
        std::mem::swap(&mut ret, &mut self.account_state_gens);
        ret
    }
}

impl Arbitrary for TransactionToCommitGen {
    type Parameters = ();

    fn arbitrary_with(_args: Self::Parameters) -> Self::Strategy {
        (
            (
                any::<Index>(),
                any::<AccountStateGen>(),
                any::<SignatureCheckedTransactionGen>(),
            ),
            vec(
                (
                    any::<Index>(),
                    any::<AccountStateGen>(),
                    any::<ContractEventGen>(),
                ),
                0..=2,
            ),
            vec((any::<Index>(), any::<AccountStateGen>()), 0..=1),
            any::<u64>(),
            any::<ExecutionStatus>(),
        )
            .prop_map(
                |(sender, event_emitters, mut touched_accounts, gas_used, status)| {
                    // To reflect change of account/event sequence numbers, txn sender account and
                    // event emitter accounts must be updated.
                    let (sender_index, sender_blob_gen, txn_gen) = sender;
                    touched_accounts.push((sender_index, sender_blob_gen));

                    let mut event_gens = Vec::new();
                    for (index, blob_gen, event_gen) in event_emitters {
                        touched_accounts.push((index, blob_gen));
                        event_gens.push((index, event_gen));
                    }

                    Self {
                        transaction_gen: (sender_index, txn_gen),
                        event_gens,
                        account_state_gens: touched_accounts,
                        gas_used,
                        status,
                    }
                },
            )
            .boxed()
    }

    type Strategy = BoxedStrategy<Self>;
}

fn arb_transaction_list_with_proof() -> impl Strategy<Value = TransactionListWithProof> {
    (
        vec(
            (
                any::<SignedTransaction>(),
                vec(any::<ContractEvent>(), 0..10),
            ),
            0..10,
        ),
        any::<TransactionInfoListWithProof>(),
    )
        .prop_flat_map(|(transaction_and_events, proof)| {
            let transactions: Vec<_> = transaction_and_events
                .clone()
                .into_iter()
                .map(|(transaction, _event)| Transaction::UserTransaction(transaction))
                .collect();
            let events: Vec<_> = transaction_and_events
                .into_iter()
                .map(|(_transaction, event)| event)
                .collect();

            (
                Just(transactions.clone()),
                option::of(Just(events)),
                if transactions.is_empty() {
                    Just(None).boxed()
                } else {
                    any::<Version>().prop_map(Some).boxed()
                },
                Just(proof),
            )
        })
        .prop_map(|(transactions, events, first_txn_version, proof)| {
            TransactionListWithProof::new(transactions, events, first_txn_version, proof)
        })
}

impl Arbitrary for TransactionListWithProof {
    type Parameters = ();
    fn arbitrary_with(_args: Self::Parameters) -> Self::Strategy {
        arb_transaction_list_with_proof().boxed()
    }

    type Strategy = BoxedStrategy<Self>;
}

impl Arbitrary for BlockMetadata {
    type Parameters = SizeRange;
    fn arbitrary_with(num_validators_range: Self::Parameters) -> Self::Strategy {
        (
            any::<HashValue>(),
            any::<u64>(),
            any::<u64>(),
            any::<AccountAddress>(),
            prop::collection::vec(any::<u8>(), num_validators_range.clone()),
            prop::collection::vec(any::<u32>(), num_validators_range),
            any::<u64>(),
        )
            .prop_map(
                |(
                    id,
                    epoch,
                    round,
                    proposer,
                    previous_block_votes,
                    failed_proposer_indices,
                    timestamp,
                )| {
                    BlockMetadata::new(
                        id,
                        epoch,
                        round,
                        proposer,
                        previous_block_votes,
                        failed_proposer_indices,
                        timestamp,
                    )
                },
            )
            .boxed()
    }

    type Strategy = BoxedStrategy<Self>;
}

#[derive(Debug)]
struct ValidatorSetGen {
    validators: Vec<Index>,
}

impl ValidatorSetGen {
    pub fn materialize(self, universe: &mut AccountInfoUniverse) -> Vec<ValidatorSigner> {
        universe
            .get_account_infos_dedup(&self.validators)
            .iter()
            .map(|account| {
                ValidatorSigner::new(account.address, account.consensus_private_key.clone())
            })
            .collect()
    }
}

impl Arbitrary for ValidatorSetGen {
    type Parameters = ();
    fn arbitrary_with(_args: Self::Parameters) -> Self::Strategy {
        vec(any::<Index>(), 1..3)
            .prop_map(|validators| Self { validators })
            .boxed()
    }

    type Strategy = BoxedStrategy<Self>;
}

#[derive(Debug)]
pub struct BlockInfoGen {
    id: HashValue,
    executed_state_id: HashValue,
    timestamp_usecs: u64,
    new_epoch: bool,
    validator_set_gen: ValidatorSetGen,
}

impl BlockInfoGen {
    pub fn materialize(self, universe: &mut AccountInfoUniverse, block_size: usize) -> BlockInfo {
        assert!(block_size > 0, "No empty blocks are allowed.");

        let current_epoch = universe.get_epoch();
        // The first LedgerInfo should always carry a validator set.
        let next_epoch_state = if current_epoch == 0 || self.new_epoch {
            let next_validator_set = self.validator_set_gen.materialize(universe);
            let next_validator_infos = next_validator_set
                .iter()
                .enumerate()
                .map(|(index, signer)| {
                    ValidatorInfo::new_with_test_network_keys(
                        signer.author(),
                        signer.public_key(),
                        1, /* consensus_voting_power */
                        index as u64,
                    )
                })
                .collect();
            let next_epoch_state = EpochState {
                epoch: current_epoch + 1,
                verifier: (&ValidatorSet::new(next_validator_infos)).into(),
            };

            universe.get_and_bump_epoch();
            universe.set_validator_set(current_epoch + 1, next_validator_set);
            Some(next_epoch_state)
        } else {
            None
        };

        BlockInfo::new(
            current_epoch,
            universe.get_and_bump_round(),
            self.id,
            self.executed_state_id,
            universe.bump_and_get_version(block_size),
            self.timestamp_usecs,
            next_epoch_state,
        )
    }
}

impl Arbitrary for BlockInfoGen {
    type Parameters = ();
    type Strategy = BoxedStrategy<Self>;

    fn arbitrary_with(_args: Self::Parameters) -> Self::Strategy {
        // A small percent of them generate epoch changes.
        (
            any::<HashValue>(),
            any::<HashValue>(),
            any::<u64>(),
            prop_oneof![1 => Just(true), 3 => Just(false)],
            any::<ValidatorSetGen>(),
        )
            .prop_map(
                |(id, executed_state_id, timestamp_usecs, new_epoch, validator_set_gen)| Self {
                    id,
                    executed_state_id,
                    timestamp_usecs,
                    new_epoch,
                    validator_set_gen,
                },
            )
            .boxed()
    }
}

#[derive(Arbitrary, Debug)]
pub struct LedgerInfoGen {
    commit_info_gen: BlockInfoGen,
    consensus_data_hash: HashValue,
}

impl LedgerInfoGen {
    pub fn materialize(self, universe: &mut AccountInfoUniverse, block_size: usize) -> LedgerInfo {
        LedgerInfo::new(
            self.commit_info_gen.materialize(universe, block_size),
            self.consensus_data_hash,
        )
    }
}

#[derive(Debug)]
pub struct BlockGen {
    txn_gens: Vec<TransactionToCommitGen>,
    ledger_info_gen: LedgerInfoGen,
}

impl Arbitrary for BlockGen {
    type Parameters = usize;
    type Strategy = BoxedStrategy<Self>;

    fn arbitrary_with(max_user_txns: Self::Parameters) -> Self::Strategy {
        assert!(max_user_txns >= 1);
        (
            vec(any::<TransactionToCommitGen>(), 1..=max_user_txns),
            any::<LedgerInfoGen>(),
        )
            .prop_map(|(txn_gens, ledger_info_gen)| Self {
                txn_gens,
                ledger_info_gen,
            })
            .boxed()
    }
}

impl BlockGen {
    pub fn materialize(
        self,
        universe: &mut AccountInfoUniverse,
    ) -> (Vec<TransactionToCommit>, LedgerInfo) {
        let mut txns_to_commit = Vec::new();

        // materialize user transactions
        for txn_gen in self.txn_gens {
            txns_to_commit.push(txn_gen.materialize(universe));
        }

        txns_to_commit.push(TransactionToCommit::new(
            Transaction::StateCheckpoint(HashValue::random()),
            TransactionInfo::new_placeholder(
                0,
                Some(HashValue::random()),
                ExecutionStatus::Success,
            ),
            HashMap::new(),
            WriteSet::default(),
            Vec::new(),
            false,
        ));

        // materialize ledger info
        let ledger_info = self
            .ledger_info_gen
            .materialize(universe, txns_to_commit.len());

        (txns_to_commit, ledger_info)
    }
}

#[derive(Debug)]
pub struct LedgerInfoWithSignaturesGen {
    ledger_info_gen: LedgerInfoGen,
}

impl Arbitrary for LedgerInfoWithSignaturesGen {
    type Parameters = ();
    type Strategy = BoxedStrategy<Self>;

    fn arbitrary_with(_args: Self::Parameters) -> Self::Strategy {
        any::<LedgerInfoGen>()
            .prop_map(|ledger_info_gen| LedgerInfoWithSignaturesGen { ledger_info_gen })
            .boxed()
    }
}

impl LedgerInfoWithSignaturesGen {
    pub fn materialize(
        self,
        universe: &mut AccountInfoUniverse,
        block_size: usize,
    ) -> LedgerInfoWithSignatures {
        let ledger_info = self.ledger_info_gen.materialize(universe, block_size);
        generate_ledger_info_with_sig(universe.get_validator_set(ledger_info.epoch()), ledger_info)
    }
}

// This function generates an arbitrary serde_json::Value.
pub fn arb_json_value() -> impl Strategy<Value = Value> {
    let leaf = prop_oneof![
        Just(Value::Null),
        any::<bool>().prop_map(Value::Bool),
        any::<f64>().prop_map(|n| serde_json::json!(n)),
        any::<String>().prop_map(Value::String),
    ];

    leaf.prop_recursive(
        10,  // 10 levels deep
        256, // Maximum size of 256 nodes
        10,  // Up to 10 items per collection
        |inner| {
            prop_oneof![
                prop::collection::vec(inner.clone(), 0..10).prop_map(Value::Array),
                prop::collection::hash_map(any::<String>(), inner, 0..10)
                    .prop_map(|map| serde_json::json!(map)),
            ]
        },
    )
}

impl Arbitrary for ValidatorVerifier {
    type Parameters = ();

    fn arbitrary_with(_args: Self::Parameters) -> Self::Strategy {
        vec(any::<ValidatorConsensusInfo>(), 1..1000)
            .prop_map(ValidatorVerifier::new)
            .boxed()
    }

    type Strategy = BoxedStrategy<Self>;
}
