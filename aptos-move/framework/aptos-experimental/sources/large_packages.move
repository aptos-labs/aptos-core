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
/// The Aptos CLI implements this flow with the `--chunked-publish` flag:
/// - `aptos move publish [OPTIONS] --chunked-publish`
/// - `aptos move create-object-and-publish-package [OPTIONS] --address-name <ADDRESS_NAME> --chunked-publish`
/// - `aptos move upgrade-object-package [OPTIONS] --object-address <OBJECT_ADDRESS> --chunked-publish`
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
/// * This module is deployed at `0x7` (aptos-experimental) on networks whose genesis includes it, such
///   as testnet, devnet and localnet. On mainnet, use the deployment at
///   `0x0e1ca3011bdd07246d4d16d909dbb2d6953a86c4735d5acf5865d962c630cce7`.
/// * Ensure that `code_indices` have no gaps. Publishing assembles the modules at indices `0` through
///   `StagingArea.last_module_idx` (the highest index staged), so if any index in that range was never
///   staged, e.g. `[0, 1, 3]`, the final call aborts with `EMISSING_MODULE_CODE`.
module aptos_experimental::large_packages {
    use std::error;
    use std::signer;
    use aptos_std::smart_table::{Self, SmartTable};

    use aptos_framework::code::{Self, PackageRegistry};
    use aptos_framework::object::{Object};
    use aptos_framework::object_code_deployment;

    /// code_indices and code_chunks should be the same length.
    const ECODE_MISMATCH: u64 = 1;
    /// Code is missing for at least one module index between 0 and the highest staged index.
    const EMISSING_MODULE_CODE: u64 = 3;

    struct StagingArea has key {
        metadata_serialized: vector<u8>,
        code: SmartTable<u64, vector<u8>>,
        last_module_idx: u64
    }

    public entry fun stage_code_chunk(
        owner: &signer,
        metadata_chunk: vector<u8>,
        code_indices: vector<u16>,
        code_chunks: vector<vector<u8>>
    ) acquires StagingArea {
        stage_code_chunk_internal(
            owner,
            metadata_chunk,
            code_indices,
            code_chunks
        );
    }

    public entry fun stage_code_chunk_and_publish_to_account(
        owner: &signer,
        metadata_chunk: vector<u8>,
        code_indices: vector<u16>,
        code_chunks: vector<vector<u8>>
    ) acquires StagingArea {
        let staging_area =
            stage_code_chunk_internal(
                owner,
                metadata_chunk,
                code_indices,
                code_chunks
            );
        publish_to_account(owner, staging_area);
        cleanup_staging_area(owner);
    }

    public entry fun stage_code_chunk_and_publish_to_object(
        owner: &signer,
        metadata_chunk: vector<u8>,
        code_indices: vector<u16>,
        code_chunks: vector<vector<u8>>
    ) acquires StagingArea {
        let staging_area =
            stage_code_chunk_internal(
                owner,
                metadata_chunk,
                code_indices,
                code_chunks
            );
        publish_to_object(owner, staging_area);
        cleanup_staging_area(owner);
    }

    public entry fun stage_code_chunk_and_upgrade_object_code(
        owner: &signer,
        metadata_chunk: vector<u8>,
        code_indices: vector<u16>,
        code_chunks: vector<vector<u8>>,
        code_object: Object<PackageRegistry>
    ) acquires StagingArea {
        let staging_area =
            stage_code_chunk_internal(
                owner,
                metadata_chunk,
                code_indices,
                code_chunks
            );
        upgrade_object_code(owner, staging_area, code_object);
        cleanup_staging_area(owner);
    }

    inline fun stage_code_chunk_internal(
        owner: &signer,
        metadata_chunk: vector<u8>,
        code_indices: vector<u16>,
        code_chunks: vector<vector<u8>>
    ): &mut StagingArea {
        assert!(
            code_indices.length() == code_chunks.length(),
            error::invalid_argument(ECODE_MISMATCH)
        );

        let owner_address = signer::address_of(owner);

        if (!exists<StagingArea>(owner_address)) {
            move_to(
                owner,
                StagingArea {
                    metadata_serialized: vector[],
                    code: smart_table::new(),
                    last_module_idx: 0
                }
            );
        };

        let staging_area = borrow_global_mut<StagingArea>(owner_address);

        if (!metadata_chunk.is_empty()) {
            staging_area.metadata_serialized.append(metadata_chunk);
        };

        let i = 0;
        while (i < code_chunks.length()) {
            let inner_code = code_chunks[i];
            let idx = (code_indices[i] as u64);

            if (staging_area.code.contains(idx)) {
                staging_area.code.borrow_mut(idx).append(inner_code);
            } else {
                staging_area.code.add(idx, inner_code);
                if (idx > staging_area.last_module_idx) {
                    staging_area.last_module_idx = idx;
                }
            };
            i += 1;
        };

        staging_area
    }

    inline fun publish_to_account(
        publisher: &signer, staging_area: &mut StagingArea
    ) {
        let code = assemble_module_code(staging_area);
        code::publish_package_txn(publisher, staging_area.metadata_serialized, code);
    }

    inline fun publish_to_object(
        publisher: &signer, staging_area: &mut StagingArea
    ) {
        let code = assemble_module_code(staging_area);
        object_code_deployment::publish(
            publisher, staging_area.metadata_serialized, code
        );
    }

    inline fun upgrade_object_code(
        publisher: &signer,
        staging_area: &mut StagingArea,
        code_object: Object<PackageRegistry>
    ) {
        let code = assemble_module_code(staging_area);
        object_code_deployment::upgrade(
            publisher,
            staging_area.metadata_serialized,
            code,
            code_object
        );
    }

    inline fun assemble_module_code(staging_area: &mut StagingArea): vector<vector<u8>> {
        let last_module_idx = staging_area.last_module_idx;
        // Staged indices are unique and never exceed `last_module_idx`, so a matching count means
        // every index from 0 to `last_module_idx` is present.
        assert!(
            staging_area.code.length() == last_module_idx + 1,
            error::invalid_state(EMISSING_MODULE_CODE)
        );
        let code = vector[];
        let i = 0;
        while (i <= last_module_idx) {
            code.push_back(*staging_area.code.borrow(i));
            i += 1;
        };
        code
    }

    public entry fun cleanup_staging_area(owner: &signer) acquires StagingArea {
        let StagingArea { metadata_serialized: _, code, last_module_idx: _ } =
            move_from<StagingArea>(signer::address_of(owner));
        code.destroy();
    }

    #[test(owner = @0x123)]
    fun test_stage_code_chunks_and_cleanup(owner: &signer) acquires StagingArea {
        stage_code_chunk(
            owner,
            x"0102",
            vector[0, 1],
            vector[x"aa", x"bb"]
        );
        // A later chunk for an already staged module is appended to it.
        stage_code_chunk(owner, x"03", vector[0], vector[x"cc"]);

        let staging_area = borrow_global<StagingArea>(signer::address_of(owner));
        assert!(staging_area.metadata_serialized == x"010203");
        assert!(staging_area.last_module_idx == 1);
        assert!(*staging_area.code.borrow(0) == x"aacc");
        assert!(*staging_area.code.borrow(1) == x"bb");

        cleanup_staging_area(owner);
        assert!(
            !exists<StagingArea>(signer::address_of(owner))
        );
    }

    #[test(owner = @0x123)]
    #[expected_failure(abort_code = 0x30003, location = Self)]
    fun test_publish_with_gap_in_module_indices_fails(owner: &signer) acquires StagingArea {
        stage_code_chunk(owner, x"01", vector[0, 2], vector[x"aa", x"bb"]);
        stage_code_chunk_and_publish_to_account(owner, vector[], vector[], vector[]);
    }

    #[test(owner = @0x123)]
    #[expected_failure(abort_code = 0x30003, location = Self)]
    fun test_publish_without_staged_code_fails(owner: &signer) acquires StagingArea {
        stage_code_chunk_and_publish_to_account(owner, x"01", vector[], vector[]);
    }
}
