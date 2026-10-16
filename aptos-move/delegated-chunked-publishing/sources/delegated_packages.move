/// Standalone position-keyed chunk staging. Each area belongs to the signing stager;
/// its designated publisher cannot be changed while that area exists.
module delegated_packages::delegated_packages {
    use std::bcs;
    use std::error;
    use std::hash;
    use std::signer;
    use std::vector;
    use aptos_std::table_with_length::{Self, TableWithLength};
    use aptos_framework::code;
    use aptos_framework::object::Object;
    use aptos_framework::object_code_deployment;

    #[test_only]
    use aptos_framework::account;
    #[test_only]
    use aptos_framework::object;
    #[test_only]
    use std::string;
    #[test_only]
    use aptos_framework::event;

    /// No staging area exists for the signing stager (not found, 0x60001).
    const ENOT_OPEN: u64 = 1;
    /// A staged position already contains different bytes (aborted, 0x70002).
    const ECHUNK_CONFLICT: u64 = 2;
    /// The stager already has an area, possibly for another publisher (already exists, 0x80003).
    const EPUBLISHER_MISMATCH: u64 = 3;
    /// Chunk positions and payloads have different lengths (invalid argument, 0x10004).
    const ELENGTH_MISMATCH: u64 = 4;

    /// Signer does not match the publisher bound when the area was opened.
    const EWRONG_PUBLISHER: u64 = 5;
    /// The expected SHA3-256 digest is not the digest of the package preimage.
    const EHASH_MISMATCH: u64 = 6;
    /// At least one required chunk or part is missing.
    const EMISSING_CHUNK: u64 = 7;
    /// Staged chunks are not accounted for by the declared part counts.
    const EUNEXPECTED_CHUNKS: u64 = 8;

    /// BCS encoding of this struct commits to the ordered metadata and module bytes.
    struct PackagePreimage has drop {
        metadata: vector<u8>,
        code: vector<vector<u8>>,
    }

    struct StagingArea has key {
        publisher: address,
        chunks: TableWithLength<u64, vector<u8>>,
        keys: vector<u64>,
    }

    /// Open a staging area at the signing stager's account, binding it to `publisher`.
    /// An existing area must be explicitly cleaned up before opening another one.
    public entry fun open_staging(stager: &signer, publisher: address) {
        let stager_address = signer::address_of(stager);
        assert!(!exists<StagingArea>(stager_address), error::already_exists(EPUBLISHER_MISMATCH));
        move_to(stager, StagingArea {
            publisher,
            chunks: table_with_length::new(),
            keys: vector[],
        });
    }

    /// Stage bytes at (part, index) positions owned by the signing stager.
    /// Identical replays are no-ops; different bytes at the same position abort.
    public entry fun stage_chunks(
        stager: &signer,
        part_indices: vector<u16>,
        chunk_indices: vector<u16>,
        chunks: vector<vector<u8>>,
    ) acquires StagingArea {
        let count = vector::length(&chunks);
        assert!(vector::length(&part_indices) == count && vector::length(&chunk_indices) == count,
            error::invalid_argument(ELENGTH_MISMATCH));
        let stager_address = signer::address_of(stager);
        assert!(exists<StagingArea>(stager_address), error::not_found(ENOT_OPEN));
        let area = borrow_global_mut<StagingArea>(stager_address);
        let i = 0;
        while (i < count) {
            let key = ((*vector::borrow(&part_indices, i) as u64) << 32)
                | (*vector::borrow(&chunk_indices, i) as u64);
            let bytes = vector::borrow(&chunks, i);
            if (table_with_length::contains(&area.chunks, key)) {
                assert!(table_with_length::borrow(&area.chunks, key) == bytes,
                    error::aborted(ECHUNK_CONFLICT));
            } else {
                table_with_length::add(&mut area.chunks, key, *bytes);
                vector::push_back(&mut area.keys, key);
            };
            i = i + 1;
        };
    }

    /// Delete only the signing stager's area and all its table entries; absent is a no-op.
    public entry fun cleanup_staging(stager: &signer) acquires StagingArea {
        let stager_address = signer::address_of(stager);
        if (!exists<StagingArea>(stager_address)) return;
        let StagingArea { publisher: _, chunks, keys } = move_from<StagingArea>(stager_address);
        while (!vector::is_empty(&keys)) {
            let key = vector::pop_back(&mut keys);
            table_with_length::remove(&mut chunks, key);
        };
        table_with_length::destroy_empty(chunks);
    }

    /// Assemble a staged package only for its designated publisher, verify its exact
    /// contents and digest, and publish at the publisher's account. Any abort rolls
    /// back removal of the area and its chunks.
    public entry fun finalize_to_account(
        publisher: &signer,
        stager: address,
        counts: vector<u16>,
        expected_hash: vector<u8>,
    ) acquires StagingArea {
        let (metadata, code) =
            take_verified(stager, signer::address_of(publisher), counts, expected_hash);
        code::publish_package_txn(publisher, metadata, code);
    }

    /// Verify the staged package and publish it to a new code object owned by the
    /// signing publisher. Any abort rolls back removal of the staged area.
    public entry fun finalize_to_object(
        publisher: &signer,
        stager: address,
        counts: vector<u16>,
        expected_hash: vector<u8>,
    ) acquires StagingArea {
        let (metadata, code) =
            take_verified(stager, signer::address_of(publisher), counts, expected_hash);
        object_code_deployment::publish(publisher, metadata, code);
    }

    /// Verify the staged package and upgrade a code object owned by the signing
    /// publisher. Any abort (including ownership failure) rolls back staging.
    public entry fun finalize_upgrade(
        publisher: &signer,
        stager: address,
        counts: vector<u16>,
        expected_hash: vector<u8>,
        code_object: Object<code::PackageRegistry>,
    ) acquires StagingArea {
        let (metadata, code) =
            take_verified(stager, signer::address_of(publisher), counts, expected_hash);
        object_code_deployment::upgrade(publisher, metadata, code, code_object);
    }

    /// Consume the staged area after checking publisher, complete parts, exact
    /// membership and SHA3-256(BCS(PackagePreimage { metadata, code })).
    fun take_verified(
        stager: address,
        publisher: address,
        counts: vector<u16>,
        expected_hash: vector<u8>,
    ): (vector<u8>, vector<vector<u8>>) acquires StagingArea {
        assert!(exists<StagingArea>(stager), error::not_found(ENOT_OPEN));
        assert!(borrow_global<StagingArea>(stager).publisher == publisher,
            error::permission_denied(EWRONG_PUBLISHER));
        assert!(vector::length(&counts) > 1, error::invalid_state(EMISSING_CHUNK));
        let StagingArea { publisher: _, chunks, keys: _ } = move_from<StagingArea>(stager);
        let metadata = take_part(&mut chunks, 0, *vector::borrow(&counts, 0));
        let code = vector[];
        let part = 1;
        while (part < vector::length(&counts)) {
            vector::push_back(&mut code,
                take_part(&mut chunks, part, *vector::borrow(&counts, part)));
            part = part + 1;
        };
        assert!(table_with_length::length(&chunks) == 0, error::invalid_state(EUNEXPECTED_CHUNKS));
        table_with_length::destroy_empty(chunks);
        let PackagePreimage { metadata, code } = {
            let preimage = PackagePreimage { metadata, code };
            assert!(vector::length(&expected_hash) == 32 &&
                hash::sha3_256(bcs::to_bytes(&preimage)) == expected_hash,
                error::invalid_argument(EHASH_MISMATCH));
            preimage
        };
        (metadata, code)
    }

    fun take_part(chunks: &mut TableWithLength<u64, vector<u8>>, part: u64, count: u16): vector<u8> {
        assert!(count > 0, error::invalid_state(EMISSING_CHUNK));
        let bytes = vector[];
        let index = 0;
        while (index < (count as u64)) {
            let key = (part << 32) | index;
            assert!(table_with_length::contains(chunks, key), error::invalid_state(EMISSING_CHUNK));
            vector::append(&mut bytes, table_with_length::remove(chunks, key));
            index = index + 1;
        };
        bytes
    }

    #[test_only]
    struct TestUpgradePolicy has copy, drop { policy: u8 }
    #[test_only]
    struct TestModuleMetadata has copy, drop {
        name: string::String, source: vector<u8>, source_map: vector<u8>, extension: vector<u8>,
    }
    #[test_only]
    struct TestPackageMetadata has copy, drop {
        name: string::String, upgrade_policy: TestUpgradePolicy, upgrade_number: u64,
        source_digest: string::String, manifest: vector<u8>,
        modules: vector<TestModuleMetadata>, deps: vector<u8>, extension: vector<u8>,
    }
    #[test_only]
    fun test_metadata(): vector<u8> {
        bcs::to_bytes(&TestPackageMetadata {
            name: string::utf8(b"delegated"), upgrade_policy: TestUpgradePolicy { policy: 1 },
            upgrade_number: 0, source_digest: string::utf8(b""), manifest: vector[],
            modules: vector[TestModuleMetadata {
                name: string::utf8(b"module"), source: vector[], source_map: vector[], extension: vector[],
            }], deps: vector[], extension: vector[],
        })
    }

    #[test(stager = @0xa, publisher = @0xb)]
    #[expected_failure(abort_code = 0x50005, location = Self)]
    fun test_wrong_publisher(stager: &signer, publisher: &signer) {
        open_staging(stager, @0xc);
        finalize_to_account(publisher, @0xa, vector[1u16, 1u16], vector[]);
    }

    #[test(publisher = @0xb)]
    #[expected_failure(abort_code = 0x60001, location = Self)]
    fun test_finalize_requires_open(publisher: &signer) {
        finalize_to_account(publisher, @0xa, vector[1u16, 1u16], vector[]);
    }

    #[test(stager = @0xa, publisher = @0xb)]
    #[expected_failure(abort_code = 0x30007, location = Self)]
    fun test_missing_metadata_part(stager: &signer, publisher: &signer) {
        open_staging(stager, @0xb);
        finalize_to_account(publisher, @0xa, vector[], vector[]);
    }

    #[test(stager = @0xa, publisher = @0xb)]
    #[expected_failure(abort_code = 0x30007, location = Self)]
    fun test_missing_module_part(stager: &signer, publisher: &signer) {
        open_staging(stager, @0xb);
        finalize_to_account(publisher, @0xa, vector[1u16], vector[]);
    }

    #[test(stager = @0xa, publisher = @0xb)]
    #[expected_failure(abort_code = 0x30007, location = Self)]
    fun test_zero_count(stager: &signer, publisher: &signer) {
        open_staging(stager, @0xb);
        finalize_to_account(publisher, @0xa, vector[1u16, 0u16], vector[]);
    }

    #[test(stager = @0xa, publisher = @0xb)]
    #[expected_failure(abort_code = 0x30007, location = Self)]
    fun test_missing_chunk(stager: &signer, publisher: &signer) {
        open_staging(stager, @0xb);
        stage_chunks(stager, vector[0u16, 1u16], vector[0u16, 1u16], vector[b"meta", b"tail"]);
        finalize_to_account(publisher, @0xa, vector[1u16, 2u16], vector[]);
    }

    #[test(stager = @0xa, publisher = @0xb)]
    #[expected_failure(abort_code = 0x30008, location = Self)]
    fun test_extra_chunk(stager: &signer, publisher: &signer) {
        open_staging(stager, @0xb);
        stage_chunks(stager, vector[0u16, 1u16, 2u16], vector[0u16, 0u16, 0u16],
            vector[b"meta", b"code", b"extra"]);
        finalize_to_account(publisher, @0xa, vector[1u16, 1u16], vector[]);
    }

    #[test(stager = @0xa, publisher = @0xb)]
    #[expected_failure(abort_code = 0x10006, location = Self)]
    fun test_short_digest(stager: &signer, publisher: &signer) {
        open_staging(stager, @0xb);
        stage_chunks(stager, vector[0u16, 1u16], vector[0u16, 0u16], vector[b"meta", b"code"]);
        finalize_to_account(publisher, @0xa, vector[1u16, 1u16], vector[]);
    }

    #[test(stager = @0xa, publisher = @0xb)]
    #[expected_failure(abort_code = 0x10006, location = Self)]
    fun test_bad_digest(stager: &signer, publisher: &signer) {
        open_staging(stager, @0xb);
        stage_chunks(stager, vector[0u16, 1u16], vector[0u16, 0u16], vector[b"meta", b"code"]);
        finalize_to_account(publisher, @0xa, vector[1u16, 1u16], x"0000000000000000000000000000000000000000000000000000000000000000");
    }

    #[test(stager = @0xa, publisher = @0xb)]
    fun test_delegated_account_publish(stager: &signer, publisher: &signer) {
        let metadata = test_metadata();
        let split = vector::length(&metadata) / 2;
        open_staging(stager, @0xb);
        stage_chunks(stager, vector[1u16, 0u16, 0u16], vector[0u16, 1u16, 0u16],
            vector[b"code", vector::slice(&metadata, split, vector::length(&metadata)),
                vector::slice(&metadata, 0, split)]);
        let digest = hash::sha3_256(bcs::to_bytes(&PackagePreimage {
            metadata, code: vector[b"code"],
        }));
        finalize_to_account(publisher, @0xa, vector[2u16, 1u16], digest);
        assert!(!exists<StagingArea>(@0xa), 0);
        assert!(vector::length(&event::emitted_events<code::PublishPackage>()) == 1, 1);
    }

    // Independent SHA3-256 fixture for BCS: 04 6d657461 01 04 636f6465.
    #[test(stager = @0xa)]
    fun test_known_preimage_digest(stager: &signer) {
        open_staging(stager, @0xb);
        stage_chunks(stager, vector[1u16, 0u16], vector[0u16, 0u16], vector[b"code", b"meta"]);
        let (metadata, code) = take_verified(@0xa, @0xb, vector[1u16, 1u16],
            x"438f72562b990e09cb3383e87ea342434773e84285e85ceadc7d71dd831b812f");
        assert!(metadata == b"meta" && code == vector[b"code"], 0);
        assert!(!exists<StagingArea>(@0xa), 1);
    }

    #[test(stager = @0xa, publisher = @0xb)]
    fun test_delegated_object_publish(stager: &signer, publisher: &signer) {
        let publisher_address = signer::address_of(publisher);
        account::create_account_for_test(publisher_address);
        let seed = bcs::to_bytes(&b"aptos_framework::object_code_deployment");
        vector::append(&mut seed, bcs::to_bytes(&(account::get_sequence_number(publisher_address) + 1)));
        let object_address = object::create_object_address(&publisher_address, seed);
        let metadata = test_metadata();
        let digest = hash::sha3_256(bcs::to_bytes(&PackagePreimage {
            metadata: copy metadata, code: vector[b"code"],
        }));
        open_staging(stager, publisher_address);
        stage_chunks(stager, vector[1u16, 0u16], vector[0u16, 0u16], vector[b"code", metadata]);
        finalize_to_object(publisher, @0xa, vector[1u16, 1u16], digest);
        assert!(!exists<StagingArea>(@0xa), 0);
        assert!(object::object_exists<code::PackageRegistry>(object_address), 1);
        assert!(object::is_owner(object::address_to_object<code::PackageRegistry>(object_address),
            publisher_address), 2);
        assert!(vector::length(&event::emitted_events<code::PublishPackage>()) == 1, 3);
    }

    #[test(stager = @0xa, publisher = @0xb)]
    #[expected_failure(abort_code = 0x50005, location = Self)]
    fun test_object_publish_rejects_wrong_publisher(stager: &signer, publisher: &signer) {
        open_staging(stager, @0xc);
        finalize_to_object(publisher, @0xa, vector[1u16, 1u16], vector[]);
    }

    #[test(stager = @0xa, publisher = @0xb)]
    #[expected_failure(abort_code = 0x10006, location = Self)]
    fun test_object_publish_rejects_bad_digest(stager: &signer, publisher: &signer) {
        open_staging(stager, @0xb);
        stage_chunks(stager, vector[0u16, 1u16], vector[0u16, 0u16], vector[b"meta", b"code"]);
        finalize_to_object(publisher, @0xa, vector[1u16, 1u16],
            x"0000000000000000000000000000000000000000000000000000000000000000");
    }

    // A single Move test transaction can publish only once, so verify the ownership
    // check before the framework's transaction-level second-publication check.
    #[test(stager = @0xa, owner = @0xb, other = @0xc)]
    #[expected_failure(abort_code = 0x50002, location = aptos_framework::object_code_deployment)]
    fun test_delegated_upgrade_rejects_non_owner(
        stager: &signer, owner: &signer, other: &signer,
    ) {
        let owner_address = signer::address_of(owner);
        account::create_account_for_test(owner_address);
        let seed = bcs::to_bytes(&b"aptos_framework::object_code_deployment");
        vector::append(&mut seed, bcs::to_bytes(&(account::get_sequence_number(owner_address) + 1)));
        let object_address = object::create_object_address(&owner_address, seed);
        let metadata = test_metadata();
        let digest = hash::sha3_256(bcs::to_bytes(&PackagePreimage {
            metadata: copy metadata, code: vector[b"code"],
        }));
        open_staging(stager, owner_address);
        stage_chunks(stager, vector[0u16, 1u16], vector[0u16, 0u16], vector[metadata, b"code"]);
        finalize_to_object(owner, @0xa, vector[1u16, 1u16], digest);
        let code_object = object::address_to_object<code::PackageRegistry>(object_address);
        let metadata = test_metadata();
        let digest = hash::sha3_256(bcs::to_bytes(&PackagePreimage {
            metadata: copy metadata, code: vector[b"new code"],
        }));
        open_staging(stager, signer::address_of(other));
        stage_chunks(stager, vector[0u16, 1u16], vector[0u16, 0u16], vector[metadata, b"new code"]);
        finalize_upgrade(other, @0xa, vector[1u16, 1u16], digest, code_object);
    }

    #[test(stager = @0xa, publisher = @0xb)]
    fun test_out_of_order_and_identical_replays(stager: &signer, publisher: &signer) {
        open_staging(stager, @0xb);
        assert!(borrow_global<StagingArea>(@0xa).publisher == std::signer::address_of(publisher), 0);
        stage_chunks(stager, vector[1u16], vector[0u16], vector[b"module"]);
        stage_chunks(stager, vector[0u16], vector[0u16], vector[b"metadata"]);
        stage_chunks(stager, vector[0u16, 1u16], vector[0u16, 0u16], vector[b"metadata", b"module"]);
        let area = borrow_global<StagingArea>(@0xa);
        assert!(vector::length(&area.keys) == 2, 1);
        assert!(table_with_length::length(&area.chunks) == 2, 2);
        assert!(*table_with_length::borrow(&area.chunks, 0) == b"metadata", 3);
        assert!(*table_with_length::borrow(&area.chunks, 1u64 << 32) == b"module", 4);
        assert!(!exists<StagingArea>(@0xb), 5);
        assert!(*vector::borrow(&area.keys, 0) == 1u64 << 32, 6);
        assert!(*vector::borrow(&area.keys, 1) == 0, 7);
    }

    #[test(stager = @0xa)]
    fun test_repeated_positions_in_one_batch(stager: &signer) {
        open_staging(stager, @0xb);
        stage_chunks(stager, vector[1u16, 1u16], vector[2u16, 2u16], vector[b"same", b"same"]);
        let area = borrow_global<StagingArea>(@0xa);
        assert!(vector::length(&area.keys) == 1, 0);
        assert!(table_with_length::length(&area.chunks) == 1, 1);
        assert!(*table_with_length::borrow(&area.chunks, (1u64 << 32) | 2) == b"same", 2);
    }

    #[test(stager = @0xa, publisher = @0xb)]
    #[expected_failure(abort_code = 0x70002, location = Self)]
    fun test_conflicting_replay(stager: &signer, publisher: &signer) {
        open_staging(stager, std::signer::address_of(publisher));
        stage_chunks(stager, vector[0u16], vector[0u16], vector[b"original"]);
        stage_chunks(stager, vector[0u16], vector[0u16], vector[b"different"]);
    }

    #[test(stager = @0xa, publisher = @0xb)]
    #[expected_failure(abort_code = 0x80003, location = Self)]
    fun test_reopen_with_different_publisher(stager: &signer, publisher: &signer) {
        open_staging(stager, std::signer::address_of(publisher));
        open_staging(stager, @0xc);
    }

    #[test(stager = @0xa, publisher = @0xb)]
    #[expected_failure(abort_code = 0x80003, location = Self)]
    fun test_reopen_with_same_publisher(stager: &signer, publisher: &signer) {
        open_staging(stager, std::signer::address_of(publisher));
        open_staging(stager, @0xb);
    }

    #[test(stager = @0xa)]
    #[expected_failure(abort_code = 0x60001, location = Self)]
    fun test_stage_requires_open(stager: &signer) {
        stage_chunks(stager, vector[0u16], vector[0u16], vector[b"data"]);
    }

    #[test(stager = @0xa)]
    #[expected_failure(abort_code = 0x10004, location = Self)]
    fun test_mismatched_lengths(stager: &signer) {
        open_staging(stager, @0xb);
        stage_chunks(stager, vector[0u16], vector[], vector[b"data"]);
    }

    #[test(stager = @0xa)]
    #[expected_failure(abort_code = 0x10004, location = Self)]
    fun test_mismatched_payload_length(stager: &signer) {
        open_staging(stager, @0xb);
        stage_chunks(stager, vector[0u16], vector[0u16], vector[]);
    }

    #[test(stager = @0xa, publisher = @0xb)]
    #[expected_failure(abort_code = 0x60001, location = Self)]
    fun test_publisher_cannot_stage_under_stager(stager: &signer, publisher: &signer) {
        open_staging(stager, @0xb);
        stage_chunks(publisher, vector[0u16], vector[0u16], vector[b"data"]);
    }

    #[test(stager = @0xa, publisher = @0xb)]
    fun test_cleanup_owned_area_and_absent_noop(stager: &signer, publisher: &signer) {
        open_staging(stager, @0xb);
        stage_chunks(stager, vector[1u16, 0u16], vector[0u16, 0u16], vector[b"code", b"metadata"]);
        open_staging(publisher, @0xc);
        stage_chunks(publisher, vector[0u16], vector[3u16], vector[b"other"]);
        cleanup_staging(publisher);
        assert!(!exists<StagingArea>(@0xb), 4);
        assert!(exists<StagingArea>(@0xa), 0);
        assert!(table_with_length::length(&borrow_global<StagingArea>(@0xa).chunks) == 2, 5);
        cleanup_staging(stager);
        assert!(!exists<StagingArea>(@0xa), 1);
        cleanup_staging(stager);
        open_staging(stager, @0xc);
        assert!(borrow_global<StagingArea>(@0xa).publisher == @0xc, 2);
        assert!(table_with_length::length(&borrow_global<StagingArea>(@0xa).chunks) == 0, 3);
    }
}
