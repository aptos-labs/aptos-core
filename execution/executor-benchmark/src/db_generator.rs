// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{add_accounts_impl, create_checkpoint, transaction_executor::BENCHMARKS_BLOCK_EXECUTOR_ONCHAIN_CONFIG, PipelineConfig, StorageTestConfig};
use aptos_config::{
    config::{
        HotStateConfig, RocksdbConfigs, StorageDirPaths, BUFFERED_STATE_TARGET_ITEMS,
        DEFAULT_MAX_NUM_NODES_PER_LRU_CACHE_SHARD, NO_OP_STORAGE_PRUNER_CONFIG,
    },
    utils::get_genesis_txn,
};
use aptos_crypto::{ed25519::Ed25519PrivateKey, HashValue, PrivateKey, Uniform};
use aptos_db::AptosDB;
use aptos_executor::block_executor::BlockExecutor;
use aptos_executor::db_bootstrapper::{generate_waypoint, maybe_bootstrap};
use aptos_executor_types::BlockExecutorTrait;
use aptos_logger::info;
use aptos_sdk::types::LocalAccount;
use aptos_storage_interface::DbReaderWriter;
use aptos_types::{
    account_config::CORE_CODE_ADDRESS,
    jwks::{jwk::JWK, patch::IssuerJWK},
    keyless::{
        circuit_constants::TEST_GROTH16_SETUP,
        test_utils::{get_sample_iss, get_sample_jwk},
        Groth16VerificationKey,
    },
    on_chain_config::Features,
    transaction::{
        signature_verified_transaction::into_signature_verified_block,
        Transaction, WriteSetPayload, authenticator::AuthenticationKey,
        Script, TransactionArgument, ChangeSet,
    },
    write_set::{WriteSetMut, WriteOp},
    state_store::state_value::StateValue,
    ledger_info::{LedgerInfo, LedgerInfoWithSignatures},
    block_info::BlockInfo,
    aggregate_signature::AggregateSignature,
};
use move_core_types::{
    identifier::Identifier,
    account_address::AccountAddress as MoveAddress,
};
use bcs;
use aptos_vm::{aptos_vm::AptosVMBlockExecutor, VMBlockExecutor};
use aptos_types::state_store::state_key::StateKey;
use rand::{rngs::StdRng, SeedableRng};
use std::{fs, path::Path, sync::Arc};

fn open_db(db_dir: impl AsRef<Path>, enable_storage_sharding: bool) -> DbReaderWriter {
    let mut rocksdb_configs = RocksdbConfigs::default();
    rocksdb_configs.state_merkle_db_config.max_open_files = -1;
    rocksdb_configs.enable_storage_sharding = enable_storage_sharding;
    DbReaderWriter::new(
        AptosDB::open(
            StorageDirPaths::from_path(db_dir),
            false, /* readonly */
            NO_OP_STORAGE_PRUNER_CONFIG,
            rocksdb_configs,
            false, /* enable_indexer */
            BUFFERED_STATE_TARGET_ITEMS,
            DEFAULT_MAX_NUM_NODES_PER_LRU_CACHE_SHARD,
            None, /* internal_indexer_db */
            HotStateConfig::default(),
        )
        .expect("DB should open."),
    )
}

// Manually create an Account resource for the given address
// Account structure from the Aptos framework:
// struct Account has key, store {
//     authentication_key: vector<u8>,
//     sequence_number: u64,
//     guid_creation_num: u64,
//     coin_register_events: EventHandle<CoinRegisterEvent>,
//     key_rotation_events: EventHandle<KeyRotationEvent>,
//     rotation_capability_offer: CapabilityOffer<RotationCapability>,
//     signer_capability_offer: CapabilityOffer<SignerCapability>,
// }
fn create_account_resource(owner: aptos_types::account_address::AccountAddress, auth_key: Vec<u8>) -> StateValue {
    #[derive(serde::Serialize)]
    struct ID {
        creation_num: u64,
        addr: aptos_types::account_address::AccountAddress,
    }

    #[derive(serde::Serialize)]
    struct GUID {
        id: ID,
    }

    #[derive(serde::Serialize)]
    struct EventHandle {
        counter: u64,
        guid: GUID,
    }

    #[derive(serde::Serialize)]
    struct CapabilityOffer {
        for_addr: Option<aptos_types::account_address::AccountAddress>,
    }

    #[derive(serde::Serialize)]
    struct Account {
        authentication_key: Vec<u8>,
        sequence_number: u64,
        guid_creation_num: u64,
        coin_register_events: EventHandle,
        key_rotation_events: EventHandle,
        rotation_capability_offer: CapabilityOffer,
        signer_capability_offer: CapabilityOffer,
    }

    let account = Account {
        authentication_key: auth_key,
        sequence_number: 0,
        // Account is created FIRST and uses GUIDs 0 and 1 for its event handles
        // CoinStore (created after) uses GUIDs 2 and 3
        // So next available GUID is 4
        guid_creation_num: 4,
        coin_register_events: EventHandle {
            counter: 0,
            guid: GUID {
                id: ID {
                    creation_num: 0,
                    addr: owner,
                },
            },
        },
        key_rotation_events: EventHandle {
            counter: 0,
            guid: GUID {
                id: ID {
                    creation_num: 1,
                    addr: owner,
                },
            },
        },
        rotation_capability_offer: CapabilityOffer { for_addr: None },
        signer_capability_offer: CapabilityOffer { for_addr: None },
    };

    let serialized = bcs::to_bytes(&account).expect("Failed to serialize Account");
    StateValue::new_legacy(serialized.into())
}

// Manually create a CoinStore<AptosCoin> resource with the specified balance
// This creates a minimal valid CoinStore by manually constructing BCS bytes
fn create_coin_store_state_value(owner: aptos_types::account_address::AccountAddress, balance: u64) -> StateValue {
    // CoinStore<AptosCoin> structure in Move:
    // struct CoinStore<phantom CoinType> has key {
    //     coin: Coin<CoinType>,           // Coin { value: u64 }
    //     frozen: bool,
    //     deposit_events: EventHandle,    // EventHandle { counter: u64, guid: GUID }
    //     withdraw_events: EventHandle,   // EventHandle { counter: u64, guid: GUID }
    // }
    //
    // GUID structure:
    // struct GUID has drop, store, copy {
    //     id: ID  // ID { creation_num: u64, addr: address }
    // }

    #[derive(serde::Serialize)]
    struct ID {
        creation_num: u64,
        addr: aptos_types::account_address::AccountAddress,
    }

    #[derive(serde::Serialize)]
    struct GUID {
        id: ID,
    }

    #[derive(serde::Serialize)]
    struct EventHandle {
        counter: u64,
        guid: GUID,
    }

    #[derive(serde::Serialize)]
    struct Coin {
        value: u64,
    }

    #[derive(serde::Serialize)]
    struct CoinStore {
        coin: Coin,
        frozen: bool,
        deposit_events: EventHandle,
        withdraw_events: EventHandle,
    }

    let coin_store = CoinStore {
        coin: Coin { value: balance },
        frozen: false,
        // CoinStore is created AFTER Account, so it uses GUIDs 2 and 3
        // Account already used GUIDs 0 and 1 for its event handles
        deposit_events: EventHandle {
            counter: 0,
            guid: GUID {
                id: ID {
                    creation_num: 2,
                    addr: owner,
                },
            },
        },
        withdraw_events: EventHandle {
            counter: 0,
            guid: GUID {
                id: ID {
                    creation_num: 3,
                    addr: owner,
                },
            },
        },
    };

    let serialized = bcs::to_bytes(&coin_store).expect("Failed to serialize CoinStore");
    StateValue::new_legacy(serialized.into())
}

fn generate_and_fund_root_account_in_existing_db(
    db_dir: impl AsRef<Path>,
    enable_storage_sharding: bool,
) -> LocalAccount {
    info!("Generating new root account and funding it with combined Direct WriteSet transaction...");

    let db = open_db(&db_dir, enable_storage_sharding);

    // Generate a new account with deterministic seed for reproducibility
    let mut rng = StdRng::from_seed([0u8; 32]);
    let private_key = Ed25519PrivateKey::generate(&mut rng);
    let public_key = private_key.public_key();
    let auth_key = AuthenticationKey::ed25519(&public_key);
    let address = auth_key.account_address();

    info!("Generated new root account at address: {:?}", address);

    // Compiled Move script that creates account and triggers reconfiguration
    const MINT_AND_RECONFIGURE_SCRIPT: &[u8] = include_bytes!("../scripts/build/ExecutorBenchmarkScripts/bytecode_scripts/mint_and_reconfigure.mv");

    let current_time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_micros() as u64;

    info!("Using timestamp {} for WriteSet transaction", current_time);

    // Create the script with the target address and timestamp as arguments
    let script = Script::new(
        MINT_AND_RECONFIGURE_SCRIPT.to_vec(),
        vec![],
        vec![
            TransactionArgument::Address(address),
            TransactionArgument::U64(current_time),
        ],
    );

    // Step 1: Execute the script to get the ChangeSet (without committing)
    info!("Executing Move script to get ChangeSet for account creation and epoch change...");

    // Log version before script execution
    match db.reader.get_latest_ledger_info_version() {
        Ok(version) => info!("*** DB version BEFORE script execution: {}", version),
        Err(e) => info!("*** DB version BEFORE script execution: Not available (fresh checkpoint): {:?}", e),
    }

    let script_writeset_txn = Transaction::GenesisTransaction(WriteSetPayload::Script {
        execute_as: CORE_CODE_ADDRESS,
        script,
    });

    let executor = BlockExecutor::<AptosVMBlockExecutor>::new(db.clone());
    let parent_block_id = executor.committed_block_id();
    let block_id = HashValue::random();

    executor
        .execute_and_update_state(
            (block_id, into_signature_verified_block(vec![script_writeset_txn])).into(),
            parent_block_id,
            BENCHMARKS_BLOCK_EXECUTOR_ONCHAIN_CONFIG,
        )
        .expect("Failed to execute script");

    let output = executor
        .ledger_update(block_id, parent_block_id)
        .expect("Failed to ledger_update script");

    // Check version and transaction count from script execution output
    let script_version = output.expect_last_version();
    let script_txn_count = output.execution_output.to_commit.transaction_outputs.len();
    info!("*** Script execution produced {} transaction(s), would commit at version: {}", script_txn_count, script_version);

    // Extract the WriteSet and events from the transaction output
    info!("Extracting WriteSet from script execution...");
    let txn_output = &output.execution_output.to_commit.transaction_outputs[0];
    let script_write_set = txn_output.write_set();
    let script_events = txn_output.events();

    info!("Script produced {} write ops and {} events", script_write_set.as_v0().iter().count(), script_events.len());

    // Step 2: Create Account and CoinStore resources and add them to the WriteSet
    info!("Adding Account and CoinStore resources to the WriteSet...");

    use move_core_types::language_storage::StructTag;

    // Create the Account resource
    // Use bcs::to_bytes(&address) to match the Move framework's account creation
    let account_auth_key = bcs::to_bytes(&address).expect("Failed to serialize address");
    let account_value = create_account_resource(address, account_auth_key);
    let account_tag = StructTag {
        address: MoveAddress::from_hex_literal("0x1").unwrap(),
        module: Identifier::new("account").unwrap(),
        name: Identifier::new("Account").unwrap(),
        type_args: vec![],
    };
    let account_key = StateKey::resource(&address.into(), &account_tag)
        .expect("Failed to create Account state key");

    // Use a large balance but leave headroom for arithmetic in fee checking
    // Balance needs to cover: 100 seeds * 10^15 each = 10^17 total + some headroom
    // Using 10^17 * 2 = 2 * 10^17 to be safe
    let balance = 200_000_000_000_000_000u64; // 2 * 10^17

    // Create the Primary Fungible Store for APT at the derived address
    // This is required because mainnet has operations_default_to_fa_apt_store enabled,
    // so the VM checks FA balance (not CoinStore) in the prologue.
    use aptos_types::account_config::{
        FungibleStoreResource, ObjectCoreResource, ObjectGroupResource,
        primary_apt_store,
    };
    use aptos_types::event::{EventHandle, EventKey};
    use move_core_types::move_resource::MoveStructType;

    let pfs_address = primary_apt_store(address);
    info!("Primary Fungible Store address for root account: {:?}", pfs_address);

    // Build the ObjectGroup resource group containing ObjectCore + FungibleStore
    let object_core = ObjectCoreResource::new(
        address, // owner
        false,   // allow_ungated_transfer
        EventHandle::new(EventKey::new(0, pfs_address), 0),
    );
    let fungible_store = FungibleStoreResource::new(
        MoveAddress::TEN, // metadata = APT metadata at 0xa
        balance,
        false, // frozen
    );

    let mut object_group = std::collections::BTreeMap::<StructTag, Vec<u8>>::new();
    object_group.insert(
        ObjectCoreResource::struct_tag(),
        bcs::to_bytes(&object_core).expect("Failed to serialize ObjectCore"),
    );
    object_group.insert(
        FungibleStoreResource::struct_tag(),
        bcs::to_bytes(&fungible_store).expect("Failed to serialize FungibleStore"),
    );

    let pfs_group_key = StateKey::resource_group(
        &pfs_address,
        &ObjectGroupResource::struct_tag(),
    );
    let pfs_group_value = bcs::to_bytes(&object_group).expect("Failed to serialize ObjectGroup");

    // Combine the script's WriteSet with the Account and FA store writes
    // Note: Feature flags (CONCURRENT_FUNGIBLE_BALANCE, etc.) are enabled inside the
    // mint_and_reconfigure script via change_feature_flags_for_next_epoch + force_end_epoch
    let mut combined_writes: Vec<(StateKey, WriteOp)> = script_write_set.as_v0().iter().map(|(k, v)| (k.clone(), v.clone())).collect();
    combined_writes.push((account_key, WriteOp::legacy_modification(account_value.bytes().clone())));
    combined_writes.push((pfs_group_key, WriteOp::legacy_modification(pfs_group_value.into())));

    // Also deploy AptosExperimental framework to 0x7 (needed for perpdex benchmark packages)
    {
        let exp_addr = MoveAddress::from_hex_literal("0x7").unwrap();
        let modules_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../aptos-move/framework/aptos-experimental/build/AptosExperimental/bytecode_modules");
        let mut module_count = 0usize;

        for entry in std::fs::read_dir(&modules_dir)
            .unwrap_or_else(|e| panic!("Failed to read AptosExperimental modules dir {:?}: {}", modules_dir, e))
        {
            let entry = entry.expect("Failed to read dir entry");
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("mv") {
                continue;
            }
            // Skip test-only modules
            let fname = path.file_stem().unwrap().to_str().unwrap();
            if fname.starts_with("test_") {
                continue;
            }

            let code_blob = std::fs::read(&path)
                .unwrap_or_else(|e| panic!("Failed to read module {:?}: {}", path, e));
            let module = move_binary_format::CompiledModule::deserialize(&code_blob)
                .unwrap_or_else(|e| panic!("Failed to deserialize module {:?}: {:?}", path, e));
            let module_name = module.self_id().name().to_owned();
            // Re-serialize as V9 to ensure compatibility with the WriteSet
            // (the build dir may contain V10 modules from aptos-cached-packages build)
            let mut v9_blob = Vec::new();
            module.serialize_for_version(Some(9), &mut v9_blob)
                .unwrap_or_else(|e| panic!("Failed to serialize module {:?} as V9: {:?}", path, e));
            let key = StateKey::module(&exp_addr, &module_name);
            combined_writes.push((key, WriteOp::legacy_modification(v9_blob.into())));
            module_count += 1;
        }

        // Create a minimal PackageRegistry resource at 0x7 for dependency resolution
        // For policy-exempted addresses like 0x7, only exists<PackageRegistry> is checked
        let registry = aptos_framework::natives::code::PackageRegistry {
            packages: vec![],
        };
        let registry_tag = move_core_types::language_storage::StructTag {
            address: MoveAddress::from_hex_literal("0x1").unwrap(),
            module: Identifier::new("code").unwrap(),
            name: Identifier::new("PackageRegistry").unwrap(),
            type_args: vec![],
        };
        let registry_key = StateKey::resource(&exp_addr, &registry_tag)
            .expect("Failed to create PackageRegistry StateKey");
        let registry_value = bcs::to_bytes(&registry).expect("Failed to serialize PackageRegistry");
        combined_writes.push((registry_key, WriteOp::legacy_modification(registry_value.into())));

        info!("Added {} AptosExperimental module writes + PackageRegistry to combined WriteSet", module_count);
    }

    let num_writes = combined_writes.len();
    let combined_write_set = WriteSetMut::new(combined_writes)
        .freeze()
        .expect("Failed to freeze combined WriteSet");

    let combined_change_set = ChangeSet::new(combined_write_set, script_events.to_vec());

    info!("Combined WriteSet has {} write ops and {} events", num_writes, script_events.len());

    // Step 3: Drop the executor and create a new one for the Direct WriteSet
    drop(executor);

    // Check DB version after dropping executor (should be unchanged since we didn't commit)
    match db.reader.get_latest_ledger_info_version() {
        Ok(version) => info!("*** DB version AFTER dropping script executor (no commit): {}", version),
        Err(e) => info!("*** DB version AFTER dropping script executor: Not available: {:?}", e),
    }

    let executor = BlockExecutor::<AptosVMBlockExecutor>::new(db.clone());
    let parent_block_id = executor.committed_block_id();
    let final_block_id = HashValue::random();

    // Step 4: Execute the combined Direct WriteSet transaction
    info!("Executing combined Direct WriteSet transaction...");
    let direct_writeset_txn = Transaction::GenesisTransaction(WriteSetPayload::Direct(combined_change_set));

    let txn_vec = vec![direct_writeset_txn];
    info!("*** About to execute block with {} transaction(s)", txn_vec.len());
    let block = into_signature_verified_block(txn_vec);
    info!("*** Block contains {} transaction(s) after into_signature_verified_block", block.len());

    executor
        .execute_and_update_state(
            (final_block_id, block).into(),
            parent_block_id,
            BENCHMARKS_BLOCK_EXECUTOR_ONCHAIN_CONFIG,
        )
        .expect("Failed to execute Direct WriteSet");

    let final_output = executor
        .ledger_update(final_block_id, parent_block_id)
        .expect("Failed to ledger_update Direct WriteSet");

    // Check version and transaction count from Direct WriteSet execution output
    let direct_txn_count = final_output.execution_output.to_commit.transaction_outputs.len();
    info!("*** Direct WriteSet execution produced {} transaction(s)", direct_txn_count);

    // Step 5: Commit the changes
    info!("Committing WriteSet transaction...");

    use aptos_types::ledger_info::{LedgerInfo, LedgerInfoWithSignatures};
    use aptos_types::block_info::BlockInfo;
    use aptos_types::aggregate_signature::AggregateSignature;

    let root_hash = final_output.ledger_update_output.transaction_accumulator.root_hash();
    let version = final_output.expect_last_version();

    info!("*** Root account WriteSet will be committed at version: {}", version);
    info!("*** Direct WriteSet block contains {} transaction output(s)", final_output.execution_output.to_commit.transaction_outputs.len());

    // Log each transaction output in the block
    for (idx, txn_out) in final_output.execution_output.to_commit.transaction_outputs.iter().enumerate() {
        info!("  Transaction {}: {} write ops, {} events", idx, txn_out.write_set().as_v0().iter().count(), txn_out.events().len());
    }

    // Read the current epoch from the latest ledger info
    let epoch = db.reader.get_latest_ledger_info()
        .map(|li| li.ledger_info().epoch())
        .unwrap_or(0);
    info!("*** Current epoch from DB: {}", epoch);

    let block_info = BlockInfo::new(
        epoch,
        0,
        final_block_id,
        root_hash,
        version,
        0,
        None,
    );
    let ledger_info = LedgerInfo::new(block_info, HashValue::zero());
    let ledger_info_with_sigs = LedgerInfoWithSignatures::new(ledger_info, AggregateSignature::empty());

    executor
        .pre_commit_block(final_block_id)
        .expect("Failed to pre_commit_block");
    executor
        .commit_ledger(ledger_info_with_sigs)
        .expect("Failed to commit_ledger");

    drop(executor);

    // Verify DB version after commit
    let version_after_commit = db.reader.get_latest_ledger_info_version().expect("Failed to get version");
    info!("*** DB version AFTER committing Direct WriteSet: {}", version_after_commit);

    // Verify the FA store balance was written correctly
    info!("Verifying FA store balance for root account...");

    use aptos_storage_interface::state_store::state_view::db_state_view::LatestDbStateCheckpointView;
    use crate::db_access::DbAccessUtil;

    let state_view = db.reader.latest_state_checkpoint_view().expect("Failed to get state view");
    match DbAccessUtil::get_fungible_store(&address, &state_view) {
        Ok(fa_store) => {
            info!("*** VERIFIED: Root account FA store balance = {}", fa_store.balance());
            assert_eq!(fa_store.balance(), balance, "FA store balance mismatch");
        },
        Err(e) => {
            panic!("*** ERROR: Could not read FA store: {:?}", e);
        }
    }

    drop(db);

    let final_balance = 200_000_000_000_000_000u64; // 2 * 10^17
    info!(
        "Successfully created and funded root account {:?} with {} APT (2*10^17)",
        address,
        final_balance
    );

    LocalAccount::new(address, private_key, 0)
}

pub fn prepare_checkpoint_from_existing_db<V>(
    db_dir: impl AsRef<Path>,
    checkpoint_dir: impl AsRef<Path>,
    num_accounts: usize,
    init_account_balance: u64,
    block_size: usize,
    storage_test_config: StorageTestConfig,
    verify_sequence_numbers: bool,
    pipeline_config: PipelineConfig,
    init_features: Features,
    is_keyless: bool,
) -> LocalAccount
where
    V: VMBlockExecutor + 'static,
{
    println!("Existing database detected, preparing checkpoint...");
    println!("Preparing checkpoint from existing database at {}...", db_dir.as_ref().display());
    println!("Creating checkpoint at {}...", checkpoint_dir.as_ref().display());

    create_checkpoint(
        db_dir.as_ref(),
        checkpoint_dir.as_ref(),
        storage_test_config.enable_storage_sharding,
        false, // enable_indexer_grpc
    );
    println!("Checkpoint created successfully.");

    // Generate and fund a new root account in checkpoint database using Move script
    let root_account = generate_and_fund_root_account_in_existing_db(
        &checkpoint_dir,
        storage_test_config.enable_storage_sharding,
    );

    // Create and fund accounts in checkpoint database
    println!("Creating and funding {} accounts in checkpoint database...", num_accounts);

    // Save the root account key before moving into add_accounts_impl
    let root_private_key = root_account.private_key().clone();
    let root_address = root_account.address();

    add_accounts_impl::<V>(
        num_accounts,
        init_account_balance,
        block_size,
        &checkpoint_dir,
        &checkpoint_dir,
        storage_test_config,
        verify_sequence_numbers,
        pipeline_config,
        init_features,
        is_keyless,
        Some(root_account),
    );

    // Return a fresh LocalAccount with sequence number 0 (will be resynced by run_benchmark)
    LocalAccount::new(root_address, root_private_key, 0)
}

pub fn create_db_with_accounts<V>(
    num_accounts: usize,
    init_account_balance: u64,
    block_size: usize,
    db_dir: impl AsRef<Path>,
    storage_test_config: StorageTestConfig,
    verify_sequence_numbers: bool,
    pipeline_config: PipelineConfig,
    init_features: Features,
    is_keyless: bool,
) where
    V: VMBlockExecutor + 'static,
{
    (num_accounts as u64)
        .checked_mul(init_account_balance)
        .expect("num_accounts * init_account_balance above u64");

    println!("Initializing...");

    if db_dir.as_ref().exists() {
        panic!("data-dir exists already.");
    }
    // create if not exists
    fs::create_dir_all(db_dir.as_ref()).unwrap();

    bootstrap_with_genesis(
        &db_dir,
        storage_test_config.enable_storage_sharding,
        init_features.clone(),
    );

    println!(
        "Finished empty DB creation, DB dir: {}. Creating accounts now...",
        db_dir.as_ref().display()
    );

    add_accounts_impl::<V>(
        num_accounts,
        init_account_balance,
        block_size,
        &db_dir,
        &db_dir,
        storage_test_config,
        verify_sequence_numbers,
        pipeline_config,
        init_features,
        is_keyless,
        None, // Use genesis root account
    );
}

pub(crate) fn bootstrap_with_genesis(
    db_dir: impl AsRef<Path>,
    enable_storage_sharding: bool,
    init_features: Features,
) {
    let (config, _genesis_key) =
        aptos_genesis::test_utils::test_config_with_custom_onchain(Some(Arc::new(move |config| {
            config.initial_features_override = Some(init_features.clone());
            config.initial_jwks = vec![IssuerJWK {
                issuer: get_sample_iss(),
                jwk: JWK::RSA(get_sample_jwk()),
            }];
            config.keyless_groth16_vk = Some(Groth16VerificationKey::from(
                &TEST_GROTH16_SETUP.prepared_vk,
            ));
        })));

    let mut rocksdb_configs = RocksdbConfigs::default();
    rocksdb_configs.state_merkle_db_config.max_open_files = -1;
    rocksdb_configs.enable_storage_sharding = enable_storage_sharding;
    let (_db, db_rw) = DbReaderWriter::wrap(
        AptosDB::open(
            StorageDirPaths::from_path(db_dir),
            false, /* readonly */
            NO_OP_STORAGE_PRUNER_CONFIG,
            rocksdb_configs,
            false, /* enable_indexer */
            BUFFERED_STATE_TARGET_ITEMS,
            DEFAULT_MAX_NUM_NODES_PER_LRU_CACHE_SHARD,
            None, /* internal_indexer_db */
            HotStateConfig::default(),
        )
        .expect("DB should open."),
    );

    // Bootstrap db with genesis
    let waypoint =
        generate_waypoint::<AptosVMBlockExecutor>(&db_rw, get_genesis_txn(&config).unwrap())
            .unwrap();
    maybe_bootstrap::<AptosVMBlockExecutor>(&db_rw, get_genesis_txn(&config).unwrap(), waypoint)
        .unwrap();
}
