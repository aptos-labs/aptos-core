/// Standalone position-keyed chunk staging. Each area belongs to the signing stager;
/// its designated publisher cannot be changed while that area exists.
module delegated_packages::delegated_packages {
    use std::error;
    use std::signer;
    use std::vector;
    use aptos_std::table_with_length::{Self, TableWithLength};

    /// No staging area exists for the signing stager (not found, 0x60001).
    const ENOT_OPEN: u64 = 1;
    /// A staged position already contains different bytes (aborted, 0x70002).
    const ECHUNK_CONFLICT: u64 = 2;
    /// The stager already has an area, possibly for another publisher (already exists, 0x80003).
    const EPUBLISHER_MISMATCH: u64 = 3;
    /// Chunk positions and payloads have different lengths (invalid argument, 0x10004).
    const ELENGTH_MISMATCH: u64 = 4;

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
