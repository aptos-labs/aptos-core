// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use aptos_config::config::NodeConfig;
use aptos_crypto::{hash::HashValue, SigningKey};
use aptos_mempool::mocks::MockSharedMempool;
use aptos_protos::extractor::v1::Transaction as TransactionPB;
use aptos_sdk::{
    transaction_builder::TransactionFactory,
    types::{
        account_config::aptos_test_root_address, transaction::SignedTransaction, LocalAccount,
    },
};
use aptos_temppath::TempPath;
use aptos_types::{
    account_address::AccountAddress,
    block_info::BlockInfo,
    block_metadata::BlockMetadata,
    chain_id::ChainId,
    ledger_info::{LedgerInfo, LedgerInfoWithSignatures},
    transaction::{Transaction, TransactionStatus},
};
use aptos_vm::AptosVM;
use aptosdb::AptosDB;
use executor::{block_executor::BlockExecutor, db_bootstrapper};
use executor_types::BlockExecutorTrait;
use mempool_notifications::MempoolNotificationSender;
use storage_interface::DbReaderWriter;

use crate::tests::{golden_output::GoldenOutputs, pretty};
use aptos_api::{context::Context, index};
use aptos_api_types::HexEncodedBytes;
use aptos_config::keys::ConfigKey;
use aptos_crypto::ed25519::Ed25519PrivateKey;
use aptos_types::aggregated_signature::AggregatedSignature;
use bytes::Bytes;
use hyper::Response;
use rand::SeedableRng;
use serde_json::{json, Value};
use std::{boxed::Box, iter::once, sync::Arc, time::Duration};
use vm_validator::vm_validator::VMValidator;

pub fn new_test_context(test_name: &str, fake_start_time_usecs: u64) -> TestContext {
    let tmp_dir = TempPath::new();
    tmp_dir.create_as_dir().unwrap();

    let mut rng = ::rand::rngs::StdRng::from_seed([0u8; 32]);
    let builder = aptos_genesis::builder::Builder::new(
        tmp_dir.path(),
        framework::head_release_bundle().clone(),
    )
    .unwrap()
    .with_init_genesis_config(Some(Arc::new(|genesis_config| {
        genesis_config.recurring_lockup_duration_secs = 86400;
    })))
    .with_randomize_first_validator_ports(false);

    let (root_key, genesis, genesis_waypoint, validators) = builder.build(&mut rng).unwrap();
    let (validator_identity, _, _) = validators[0].get_key_objects(None).unwrap();
    let validator_owner = validator_identity.account_address.unwrap();

    let (db, db_rw) = DbReaderWriter::wrap(AptosDB::new_for_test_with_indexer(&tmp_dir));
    let ret =
        db_bootstrapper::maybe_bootstrap::<AptosVM>(&db_rw, &genesis, genesis_waypoint).unwrap();
    assert!(ret);

    let mempool = MockSharedMempool::new_in_runtime(&db_rw, VMValidator::new(db.clone()));

    TestContext::new(
        Context::new(
            ChainId::test(),
            db.clone(),
            mempool.ac_client.clone(),
            NodeConfig::default(),
        ),
        rng,
        root_key,
        validator_owner,
        Box::new(BlockExecutor::<AptosVM>::new(db_rw)),
        mempool,
        db,
        test_name.to_string(),
        fake_start_time_usecs,
    )
}

#[allow(dead_code)]
#[derive(Clone)]
pub struct TestContext {
    pub context: Context,
    pub validator_owner: AccountAddress,
    pub mempool: Arc<MockSharedMempool>,
    pub db: Arc<AptosDB>,
    rng: rand::rngs::StdRng,
    root_key: ConfigKey<Ed25519PrivateKey>,
    executor: Arc<dyn BlockExecutorTrait>,
    expect_status_code: u16,
    test_name: String,
    golden_output: Option<GoldenOutputs>,
    fake_time_usecs: u64,
}

// TODO: Remove after we add back golden
#[allow(dead_code)]
impl TestContext {
    pub fn new(
        context: Context,
        rng: rand::rngs::StdRng,
        root_key: Ed25519PrivateKey,
        validator_owner: AccountAddress,
        executor: Box<dyn BlockExecutorTrait>,
        mempool: MockSharedMempool,
        db: Arc<AptosDB>,
        test_name: String,
        fake_time_usecs: u64,
    ) -> Self {
        Self {
            context,
            rng,
            root_key: ConfigKey::new(root_key),
            validator_owner,
            executor: executor.into(),
            mempool: Arc::new(mempool),
            expect_status_code: 200,
            db,
            test_name,
            golden_output: None,
            fake_time_usecs,
        }
    }
    pub fn rng(&mut self) -> &mut rand::rngs::StdRng {
        &mut self.rng
    }

    pub fn transaction_factory(&self) -> TransactionFactory {
        TransactionFactory::new(self.context.chain_id())
    }

    pub fn root_account(&self) -> LocalAccount {
        LocalAccount::new(aptos_test_root_address(), self.root_key.private_key(), 0)
    }

    pub fn gen_account(&mut self) -> LocalAccount {
        LocalAccount::generate(self.rng())
    }

    pub fn create_user_account(&self, account: &LocalAccount) -> SignedTransaction {
        let mut tc = self.root_account();
        self.create_user_account_by(&mut tc, account)
    }

    pub fn create_user_account_by(
        &self,
        creator: &mut LocalAccount,
        account: &LocalAccount,
    ) -> SignedTransaction {
        let factory = self.transaction_factory();
        creator.sign_with_transaction_builder(
            factory
                .create_user_account(account.public_key())
                .expiration_timestamp_secs(u64::MAX),
        )
    }

    pub fn expect_status_code(&self, status_code: u16) -> TestContext {
        let mut ret = self.clone();
        ret.expect_status_code = status_code;
        ret
    }

    pub async fn commit_mempool_txns(&mut self, size: u64) {
        let txns = self.mempool.get_txns(size);
        self.commit_block(&txns).await;
        for txn in txns {
            self.mempool.remove_txn(&txn);
        }
    }

    pub async fn commit_block(&mut self, signed_txns: &[SignedTransaction]) {
        let metadata = self.new_block_metadata();
        let timestamp = metadata.timestamp_usecs();
        let txns: Vec<Transaction> = std::iter::once(Transaction::BlockMetadata(metadata.clone()))
            .chain(
                signed_txns
                    .iter()
                    .cloned()
                    .map(Transaction::UserTransaction),
            )
            .chain(once(Transaction::StateCheckpoint(metadata.id())))
            .collect();

        // Check that txn execution was successful.
        let parent_id = self.executor.committed_block_id();
        let result = self
            .executor
            .execute_block((metadata.id(), txns.clone()), parent_id)
            .unwrap();
        let mut compute_status = result.compute_status().clone();
        assert_eq!(compute_status.len(), txns.len(), "{:?}", result);
        if matches!(compute_status.last(), Some(TransactionStatus::Retry)) {
            // a state checkpoint txn can be Retry if prefixed by a write set txn
            compute_status.pop();
        }
        // But the rest of the txns must be Kept.
        for st in result.compute_status() {
            match st {
                TransactionStatus::Discard(st) => panic!("transaction is discarded: {:?}", st),
                TransactionStatus::Retry => panic!("should not retry"),
                TransactionStatus::Keep(_) => (),
            }
        }

        self.executor
            .commit_blocks(
                vec![metadata.id()],
                self.new_ledger_info(&metadata, result.root_hash(), txns.len()),
            )
            .unwrap();

        self.mempool
            .mempool_notifier
            .notify_new_commit(txns, timestamp, 1000)
            .await
            .unwrap();
    }

    fn new_block_metadata(&mut self) -> BlockMetadata {
        let round = 1;
        let id = HashValue::random_with_rng(&mut self.rng);
        // incrementing half a second every time
        self.fake_time_usecs += (Duration::from_secs(1).as_micros() / 2) as u64;
        BlockMetadata::new(
            id,
            0,
            round,
            self.validator_owner,
            Some(0),
            vec![0],
            vec![],
            self.fake_time_usecs,
        )
    }

    fn new_ledger_info(
        &self,
        metadata: &BlockMetadata,
        root_hash: HashValue,
        block_size: usize,
    ) -> LedgerInfoWithSignatures {
        let parent = self
            .context
            .get_latest_ledger_info_with_signatures()
            .unwrap();
        let epoch = parent.ledger_info().epoch();
        let version = parent.ledger_info().version() + (block_size as u64);
        let info = LedgerInfo::new(
            BlockInfo::new(
                epoch,
                metadata.round(),
                metadata.id(),
                root_hash,
                version,
                metadata.timestamp_usecs(),
                None,
            ),
            HashValue::zero(),
        );
        LedgerInfoWithSignatures::new(info, AggregatedSignature::empty())
    }

    pub async fn api_execute_entry_function(
        &mut self,
        account: &mut LocalAccount,
        module: &str,
        func: &str,
        type_args: serde_json::Value,
        args: serde_json::Value,
    ) {
        let function = json!(format!(
            "{}::{}::{}",
            account.address().to_hex_literal(),
            module,
            func
        ));
        self.api_execute_txn(
            account,
            json!({
                "type": "entry_function_payload",
                "function": function,
                "type_arguments": type_args,
                "arguments": args
            }),
        )
        .await;
    }

    pub async fn api_publish_module(&mut self, account: &mut LocalAccount, code: HexEncodedBytes) {
        self.api_execute_txn(
            account,
            json!({
                "type": "module_bundle_payload",
                "modules" : [
                    {"bytecode": code},
                ],
            }),
        )
        .await;
    }

    pub async fn api_execute_txn(&mut self, account: &mut LocalAccount, payload: Value) {
        let mut request = json!({
            "sender": account.address(),
            "sequence_number": account.sequence_number().to_string(),
            "gas_unit_price": "0",
            "max_gas_amount": "1000000",
            "expiration_timestamp_secs": "16373698888888",
            "payload": payload,
        });

        let resp = self
            .post("/transactions/signing_message", request.clone())
            .await;

        let signing_msg: HexEncodedBytes = resp["message"].as_str().unwrap().parse().unwrap();
        let sig = account
            .private_key()
            .sign_arbitrary_message(signing_msg.inner());

        let typ = "ed25519_signature";

        request["signature"] = json!({
            "type": typ,
            "public_key": HexEncodedBytes::from(account.public_key().to_bytes().to_vec()),
            "signature": HexEncodedBytes::from(sig.to_bytes().to_vec()),
        });

        self.expect_status_code(202)
            .post("/transactions", request)
            .await;
        self.commit_mempool_txns(1).await;
        *account.sequence_number_mut() += 1;
    }

    pub async fn post(&self, path: &str, body: Value) -> Value {
        self.execute(warp::test::request().method("POST").path(path).json(&body))
            .await
    }

    pub async fn reply(&self, req: warp::test::RequestBuilder) -> Response<Bytes> {
        req.reply(&self.get_routes_with_poem(address)).await
    }

    // Currently we still run our tests with warp.
    // https://github.com/aptos-labs/aptos-core/issues/2966
    pub fn get_routes_with_poem(
        &self,
        poem_address: SocketAddr,
    ) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
        let proxy = warp::path!("v1" / ..).and(reverse_proxy_filter(
            "v1".to_string(),
            format!("http://{}/v1", poem_address),
        ));
        proxy
    }

    pub async fn execute(&self, req: warp::test::RequestBuilder) -> Value {
        let resp = self.reply(req).await;

        let body = serde_json::from_slice(resp.body()).expect("response body is JSON");

        body
    }

    pub fn check_golden_output(&mut self, txns: &[TransactionPB]) {
        if self.golden_output.is_none() {
            self.golden_output = Some(GoldenOutputs::new(
                self.test_name.replace(':', "_"),
                "fh_v1",
            ));
        }

        let msg = pretty(txns);
        let re = regex::Regex::new("hash\": \".*\"").unwrap();
        let msg = re.replace_all(&msg, "hash\": \"\"");

        self.golden_output.as_ref().unwrap().log(&msg);
    }
}
