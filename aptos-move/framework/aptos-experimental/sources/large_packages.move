/// # Aptos Large Packages Framework
///
/// This module provides a framework for uploading large packages to the Aptos network, under standard
/// accounts or objects.
/// To publish using this API, you must divide your metadata and modules across multiple calls
/// into `large_packages::stage_code_chunk`.
/// In each pass, the caller pushes more code by calling `stage_code_chunk`.
/// In the final call, the caller can use `stage_code_chunk_and_publish_to_account`, `stage_code_chunk_and_publish_to_object`, or
/// `stage_code_chunk_and_upgrade_object_code` to upload the final data chunk and publish or upgrade the package on-chain.
///
/// The above logic is currently implemented in the Python
/// SDK: [`aptos-python-sdk`](https://github.com/aptos-labs/aptos-python-sdk/blob/main/aptos_sdk/package_publisher.py).
///
/// Aptos CLI supports this as well with `--chunked-publish` flag:
/// - `aptos move publish [OPTIONS] --chunked-publish`
/// - `aptos move create-object-and-publish-package [OPTIONS] --address-name <ADDRESS_NAME> --chunked-publish`
/// - `aptos move upgrade-object-package [OPTIONS] --address-name <ADDRESS_NAME> --chunked-publish`
///
/// # Usage
///
/// 1. **Stage Code Chunks**:
///     - Call `stage_code_chunk` with the appropriate metadata and code chunks.
///     - Ensure that `code_indices` are provided from `0` to `last_module_idx`, without any
///       gaps.
///
///
/// 2. **Publish or Upgrade**:
///     - In order to upload the last data chunk and publish the package, call `stage_code_chunk_and_publish_to_account` or `stage_code_chunk_and_publish_to_object`.
///
///     - For object code upgrades, call `stage_code_chunk_and_upgrade_object_code` with the argument `code_object` provided.
///
/// 3. **Cleanup**:
///     - In order to remove `StagingArea` resource from an account, call `cleanup_staging_area`.
///
/// # Notes
///
/// * Make sure LargePackages is deployed to your network of choice, you can currently find it both on
///   mainnet and testnet at `0xa29df848eebfe5d981f708c2a5b06d31af2be53bbd8ddc94c8523f4b903f7adb`, and
///   in 0x7 (aptos-experimental) on devnet/localnet.
/// * Ensure that `code_indices` have no gaps. For example, if code_indices are
///   provided as [0, 1, 3] (skipping index 2), the inline function `assemble_module_code` will abort
///   since `StagingArea.last_module_idx` is set as the max value of the provided index
///   from `code_indices`, and `assemble_module_code` will lookup the `StagingArea.code` SmartTable from
///   0 to `StagingArea.last_module_idx` in turn.
module aptos_experimental::large_packages {
    use std::error;
    use std::signer;
    use std::vector;
    use aptos_std::smart_table::{Self, SmartTable};

    use aptos_framework::code::{Self, PackageRegistry};
    use aptos_framework::object::{Object};
    use aptos_framework::object_code_deployment;

    /// code_indices and code_chunks should be the same length.
    const ECODE_MISMATCH: u64 = 1;
    /// Object reference should be provided when upgrading object code.
    const EMISSING_OBJECT_REFERENCE: u64 = 2;

    struct StagingArea has key {
        metadata_serialized: vector<u8>,
        code: SmartTable<u64, vector<u8>>,
        last_module_idx: u64,
    }

    public entry fun stage_code_chunk(
        owner: &signer,
        metadata_chunk: vector<u8>,
        code_indices: vector<u16>,
        code_chunks: vector<vector<u8>>,
    ) acquires StagingArea {
        stage_code_chunk_internal(owner, metadata_chunk, code_indices, code_chunks);
    }

    public entry fun stage_code_chunk_and_publish_to_account(
        owner: &signer,
        metadata_chunk: vector<u8>,
        code_indices: vector<u16>,
        code_chunks: vector<vector<u8>>,
    ) acquires StagingArea {
        let staging_area = stage_code_chunk_internal(owner, metadata_chunk, code_indices, code_chunks);
        publish_to_account(owner, staging_area);
        cleanup_staging_area(owner);
    }

    public entry fun stage_code_chunk_and_publish_to_object(
        owner: &signer,
        metadata_chunk: vector<u8>,
        code_indices: vector<u16>,
        code_chunks: vector<vector<u8>>,
    ) acquires StagingArea {
        let staging_area = stage_code_chunk_internal(owner, metadata_chunk, code_indices, code_chunks);
        publish_to_object(owner, staging_area);
        cleanup_staging_area(owner);
    }

    public entry fun stage_code_chunk_and_upgrade_object_code(
        owner: &signer,
        metadata_chunk: vector<u8>,
        code_indices: vector<u16>,
        code_chunks: vector<vector<u8>>,
        code_object: Object<PackageRegistry>,
    ) acquires StagingArea {
        let staging_area = stage_code_chunk_internal(owner, metadata_chunk, code_indices, code_chunks);
        upgrade_object_code(owner, staging_area, code_object);
        cleanup_staging_area(owner);
    }

    inline fun stage_code_chunk_internal(
        owner: &signer,
        metadata_chunk: vector<u8>,
        code_indices: vector<u16>,
        code_chunks: vector<vector<u8>>,
    ): &mut StagingArea acquires StagingArea {
        assert!(
            vector::length(&code_indices) == vector::length(&code_chunks),
            error::invalid_argument(ECODE_MISMATCH),
        );

        let owner_address = signer::address_of(owner);

        if (!exists<StagingArea>(owner_address)) {
            move_to(owner, StagingArea {
                metadata_serialized: vector[],
                code: smart_table::new(),
                last_module_idx: 0,
            });
        };

        let staging_area = borrow_global_mut<StagingArea>(owner_address);

        if (!vector::is_empty(&metadata_chunk)) {
            vector::append(&mut staging_area.metadata_serialized, metadata_chunk);
        };

        let i = 0;
        while (i < vector::length(&code_chunks)) {
            let inner_code = *vector::borrow(&code_chunks, i);
            let idx = (*vector::borrow(&code_indices, i) as u64);

            if (smart_table::contains(&staging_area.code, idx)) {
                vector::append(smart_table::borrow_mut(&mut staging_area.code, idx), inner_code);
            } else {
                smart_table::add(&mut staging_area.code, idx, inner_code);
                if (idx > staging_area.last_module_idx) {
                    staging_area.last_module_idx = idx;
                }
            };
            i = i + 1;
        };

        staging_area
    }

    inline fun publish_to_account(
        publisher: &signer,
        staging_area: &mut StagingArea,
    ) {
        let code = assemble_module_code(staging_area);
        code::publish_package_txn(publisher, staging_area.metadata_serialized, code);
    }

    inline fun publish_to_object(
        publisher: &signer,
        staging_area: &mut StagingArea,
    ) {
        let code = assemble_module_code(staging_area);
        object_code_deployment::publish(publisher, staging_area.metadata_serialized, code);
    }

    inline fun upgrade_object_code(
        publisher: &signer,
        staging_area: &mut StagingArea,
        code_object: Object<PackageRegistry>,
    ) {
        let code = assemble_module_code(staging_area);
        object_code_deployment::upgrade(publisher, staging_area.metadata_serialized, code, code_object);
    }

    inline fun assemble_module_code(
        staging_area: &mut StagingArea,
    ): vector<vector<u8>> {
        let last_module_idx = staging_area.last_module_idx;
        let code = vector[];
        let i = 0;
        while (i <= last_module_idx) {
            vector::push_back(
                &mut code,
                *smart_table::borrow(&staging_area.code, i)
            );
            i = i + 1;
        };
        code
    }

    public entry fun cleanup_staging_area(owner: &signer) acquires StagingArea {
        let StagingArea {
            metadata_serialized: _,
            code,
            last_module_idx: _,
        } = move_from<StagingArea>(signer::address_of(owner));
        smart_table::destroy(code);
    }
}
