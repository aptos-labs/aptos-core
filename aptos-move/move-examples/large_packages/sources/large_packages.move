/// This provides a framework for uploading large packages. In each pass, the caller pushes more
/// code by calling `stage_code`. In the last call, the caller can set the optoinal `publish` and
/// the package will be published inline, saving an extra transaction and additional storage costs.
/// Currently this module does not support modules that are larger than 63KB as that is the maximum
/// that can fit within a transaction and this framework does not split up individual modules.
module large_packages::large_packages {
    use std::error;
    use std::signer;
    use std::vector;

    use aptos_framework::code;

    /// code_indices and code_chunks should be the same length.
    const ECODE_MISMATCH: u64 = 1;

    struct StagingArea has drop, key {
        metadata_serialized: vector<u8>,
        code: vector<vector<u8>>,
    }

    public entry fun stage_code(
        owner: &signer,
        metadata_serialized: vector<u8>,
        code_indices: vector<u16>,
        code_chunks: vector<vector<u8>>,
        publish: bool,
    ) acquires StagingArea {
        assert!(
            vector::length(&code_indices) == vector::length(&code_chunks),
            error::invalid_argument(ECODE_MISMATCH),
        );

        let owner_address = signer::address_of(owner);

        if (!exists<StagingArea>(owner_address)) {
            move_to(owner, StagingArea {
              metadata_serialized: vector::empty(),
              code: vector::empty(),
            });
        };

        let staging_area = borrow_global_mut<StagingArea>(owner_address);
        vector::append(&mut staging_area.metadata_serialized, metadata_serialized);

        while (!vector::is_empty(&code_chunks)) {
            let inner_code = vector::pop_back(&mut code_chunks);
            let idx = (vector::pop_back(&mut code_indices) as u64);
            while (vector::length(&staging_area.code) <= idx) {
                vector::push_back(&mut staging_area.code, vector::empty());
            };
            let source_code = vector::borrow_mut(&mut staging_area.code, idx);
            vector::append(source_code, inner_code)
        };

        let _ = staging_area;

        if (publish) {
            publish_staged_code(owner, owner_address);
            move_from<StagingArea>(owner_address);
        }
    }

    public entry fun cleanup(owner: &signer) acquires StagingArea {
        move_from<StagingArea>(signer::address_of(owner));
    }

    /// Publish code from staging area.
    public entry fun publish_staged_code(
        publisher: &signer,
        staging_area_address: address,
    ) acquires StagingArea {
        let staging_area = borrow_global_mut<StagingArea>(staging_area_address);
        code::publish_package_txn(publisher, staging_area.metadata_serialized, staging_area.code);
    }
}
