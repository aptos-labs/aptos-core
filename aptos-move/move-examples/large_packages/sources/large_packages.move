/// This provides a framework for uploading large packages to standard accounts or objects.
/// In each pass, the caller pushes more code by calling `stage_code_chunk`.
/// In the final call, the caller can optionally set `publish_to_account`, `publish_to_object`, or `upgrade_object_code`.
/// If any of these options are set, the package will be published inline, saving an extra transaction and additional storage costs.
module large_packages::large_packages {
    use std::error;
    use std::option::{Self, Option};
    use std::signer;
    use std::vector;
    use aptos_std::smart_table::{Self, SmartTable};

    use aptos_framework::code::{Self, PackageRegistry};
    use aptos_framework::object::{Object};
    use aptos_framework::object_code_deployment;

    /// code_indices and code_chunks should be the same length.
    const ECODE_MISMATCH: u64 = 1;
    /// The publishing flags should either all be false, or only one should be true.
    const EINVALID_PUBLISHING_FLAGS: u64 = 2;
    /// Object reference should be provided when upgrading object code.
    const EMISSING_OBJECT_REFERENCE: u64 = 3;

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
        publish_to_account: bool,
        publish_to_object: bool,
        upgrade_object_code: bool,
        code_object: Option<Object<PackageRegistry>>,
    ) acquires StagingArea {
        let publish_param_count = (if (publish_to_account) 1 else 0) + (if (publish_to_object) 1 else 0) + (if (upgrade_object_code) 1 else 0);
        assert!(publish_param_count <= 1, error::invalid_argument(EINVALID_PUBLISHING_FLAGS));

        let staging_area = stage_code_chunk_internal(owner, metadata_chunk, code_indices, code_chunks);

        if (publish_to_account) {
            publish_to_standard_account(owner, staging_area);
            cleanup_staging_area(owner);
        } else if (publish_to_object) {
            publish_to_object(owner, staging_area);
            cleanup_staging_area(owner);
        } else if (upgrade_object_code) {
            assert!(option::is_some(&code_object), error::invalid_argument(EMISSING_OBJECT_REFERENCE));
            upgrade_object_code(owner, staging_area, option::extract(&mut code_object));
            cleanup_staging_area(owner);
        }
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

    inline fun publish_to_standard_account(
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
        let code: vector<vector<u8>> = vector[];
        let i: u64 = 0;
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

    /// Publishes the code from the staging area to a standard account.
    public entry fun publish_staged_code(
        publisher: &signer,
    ) acquires StagingArea {
        let staging_area = borrow_global_mut<StagingArea>(signer::address_of(publisher));
        let code = assemble_module_code(staging_area);
        code::publish_package_txn(publisher, staging_area.metadata_serialized, code);
    }

    /// Publishes the code from the staging area to an object.
    public entry fun publish_staged_code_to_object(
        publisher: &signer,
    ) acquires StagingArea {
        let staging_area = borrow_global_mut<StagingArea>(signer::address_of(publisher));
        let code = assemble_module_code(staging_area);
        object_code_deployment::publish(publisher, staging_area.metadata_serialized, code);
    }

    /// Upgrades the code in an object to the new code from the staging area.
    public entry fun upgrade_object_code_with_staged_code(
        publisher: &signer,
        code_object: Object<PackageRegistry>,
    ) acquires StagingArea {
        let staging_area = borrow_global_mut<StagingArea>(signer::address_of(publisher));
        let code = assemble_module_code(staging_area);
        object_code_deployment::upgrade(publisher, staging_area.metadata_serialized, code, code_object);
    }
}
