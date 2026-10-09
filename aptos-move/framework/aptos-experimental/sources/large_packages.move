/// # Aptos Large Packages Framework
///
/// This module provides a framework for uploading large packages to the Aptos network, under standard
/// accounts or objects. A package is staged in chunks over several transactions and then published
/// by a final call.
///
/// For staging, a package is split into parts: part 0 is the serialized `PackageMetadata` and part
/// `i + 1` is the code of module `i`, in publishing order. Each part is split into chunks, and a
/// chunk is identified by its position: its part index and its index within the part. Every staged
/// chunk is kept in its own storage slot, so staging a chunk never rewrites chunks staged earlier.
///
/// The Aptos CLI implements this flow with the `--chunked-publish` flag:
/// - `aptos move publish [OPTIONS] --chunked-publish`
/// - `aptos move create-object-and-publish-package [OPTIONS] --address-name <ADDRESS_NAME> --chunked-publish`
/// - `aptos move upgrade-object-package [OPTIONS] --object-address <OBJECT_ADDRESS> --chunked-publish`
///
/// # Usage
///
/// 1. **Stage chunks**: call `stage_chunks` with chunks and their positions, in any order and over
///    any number of transactions.
///     - Staging a chunk that is already staged changes nothing, so retrying a transaction is harmless.
///     - Staging a different chunk at a position that is taken aborts with `ECHUNK_CONFLICT`, which
///       catches data left from a different publish.
///
/// 2. **Publish or upgrade**: call `stage_chunks_and_publish_to_account`,
///    `stage_chunks_and_publish_to_object` or `stage_chunks_and_upgrade_object_code`, which stage the
///    chunks they are given and then publish. Their `chunk_counts` argument is the number of chunks
///    in each part. Publishing aborts with `EMISSING_CHUNK` if any of those chunks isn't staged, and
///    with `EUNEXPECTED_CHUNKS` if anything else is staged.
///
/// 3. **Cleanup**: to remove the `StagingArea` resource and everything staged in it, call
///    `cleanup_staging_area`. It does nothing if nothing is staged.
///
/// The `stage_code_chunk*` functions of earlier versions of this module are kept for existing
/// clients. Their chunks carry no positions: each chunk is appended to the chunks already staged
/// for its part, so chunks have to be staged in order, repeats can't be detected, and the package
/// is published from everything that is staged.
///
/// # Notes
///
/// * The module is released at `0x7` (aptos-experimental) only through genesis, so a network has the
///   version that was current when it was created. Mainnet has none at `0x7`, but has an earlier
///   version, with only the `stage_code_chunk*` functions, at
///   `0x0e1ca3011bdd07246d4d16d909dbb2d6953a86c4735d5acf5865d962c630cce7`.
/// * With the `stage_code_chunk*` functions, ensure that `code_indices` have no gaps: if a module
///   index below the highest one staged was never staged, e.g. `[0, 1, 3]`, publishing aborts with
///   `EMISSING_CHUNK`.
module aptos_experimental::large_packages {
    use std::error;
    use std::signer;
    use aptos_std::table_with_length::{Self, TableWithLength};

    use aptos_framework::code::{Self, PackageRegistry};
    use aptos_framework::object::{Object};
    use aptos_framework::object_code_deployment;

    /// The vectors describing the chunks to stage have different lengths.
    const ECODE_MISMATCH: u64 = 1;
    /// A chunk that the package needs is not staged.
    const EMISSING_CHUNK: u64 = 3;
    /// A different chunk is already staged at this position.
    const ECHUNK_CONFLICT: u64 = 4;
    /// Chunks are staged that the package being published doesn't include.
    const EUNEXPECTED_CHUNKS: u64 = 5;

    /// The part holding the package metadata. Module `i` is part `i + 1`.
    const METADATA_PART: u64 = 0;

    /// Chunks staged by an account for a package it is going to publish.
    struct StagingArea has key {
        /// The chunks, each in its own storage slot, keyed by `chunk_key(part, index)`.
        chunks: TableWithLength<u64, vector<u8>>,
        /// The keys of all staged chunks, so that they can be removed without knowing the package.
        keys: vector<u64>
    }

    /// Stages chunks of a package: `chunks[i]` is chunk `chunk_indices[i]` of part `part_indices[i]`.
    public entry fun stage_chunks(
        owner: &signer,
        part_indices: vector<u16>,
        chunk_indices: vector<u16>,
        chunks: vector<vector<u8>>
    ) acquires StagingArea {
        stage_chunks_internal(owner, part_indices, chunk_indices, chunks);
    }

    /// Stages the given chunks, then publishes the package under `owner`. `chunk_counts[p]` is the
    /// number of chunks in part `p`.
    public entry fun stage_chunks_and_publish_to_account(
        owner: &signer,
        part_indices: vector<u16>,
        chunk_indices: vector<u16>,
        chunks: vector<vector<u8>>,
        chunk_counts: vector<u16>
    ) acquires StagingArea {
        stage_chunks_internal(owner, part_indices, chunk_indices, chunks);
        let (metadata_serialized, code) =
            take_staged_package(signer::address_of(owner), chunk_counts);
        code::publish_package_txn(owner, metadata_serialized, code);
    }

    /// Stages the given chunks, then publishes the package to a new object. `chunk_counts[p]` is the
    /// number of chunks in part `p`.
    public entry fun stage_chunks_and_publish_to_object(
        owner: &signer,
        part_indices: vector<u16>,
        chunk_indices: vector<u16>,
        chunks: vector<vector<u8>>,
        chunk_counts: vector<u16>
    ) acquires StagingArea {
        stage_chunks_internal(owner, part_indices, chunk_indices, chunks);
        let (metadata_serialized, code) =
            take_staged_package(signer::address_of(owner), chunk_counts);
        object_code_deployment::publish(owner, metadata_serialized, code);
    }

    /// Stages the given chunks, then upgrades the package in `code_object`. `chunk_counts[p]` is the
    /// number of chunks in part `p`.
    public entry fun stage_chunks_and_upgrade_object_code(
        owner: &signer,
        part_indices: vector<u16>,
        chunk_indices: vector<u16>,
        chunks: vector<vector<u8>>,
        chunk_counts: vector<u16>,
        code_object: Object<PackageRegistry>
    ) acquires StagingArea {
        stage_chunks_internal(owner, part_indices, chunk_indices, chunks);
        let (metadata_serialized, code) =
            take_staged_package(signer::address_of(owner), chunk_counts);
        object_code_deployment::upgrade(owner, metadata_serialized, code, code_object);
    }

    /// Stages a chunk of the metadata, if `metadata_chunk` isn't empty, and chunks of code:
    /// `code_chunks[i]` belongs to module `code_indices[i]`. Each chunk is appended to the chunks
    /// already staged for its part.
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

    /// Like `stage_code_chunk`, then publishes the package staged so far under `owner`.
    public entry fun stage_code_chunk_and_publish_to_account(
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
        let (metadata_serialized, code) = take_all_staged(signer::address_of(owner));
        code::publish_package_txn(owner, metadata_serialized, code);
    }

    /// Like `stage_code_chunk`, then publishes the package staged so far to a new object.
    public entry fun stage_code_chunk_and_publish_to_object(
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
        let (metadata_serialized, code) = take_all_staged(signer::address_of(owner));
        object_code_deployment::publish(owner, metadata_serialized, code);
    }

    /// Like `stage_code_chunk`, then upgrades the package in `code_object` to the package staged so far.
    public entry fun stage_code_chunk_and_upgrade_object_code(
        owner: &signer,
        metadata_chunk: vector<u8>,
        code_indices: vector<u16>,
        code_chunks: vector<vector<u8>>,
        code_object: Object<PackageRegistry>
    ) acquires StagingArea {
        stage_code_chunk_internal(
            owner,
            metadata_chunk,
            code_indices,
            code_chunks
        );
        let (metadata_serialized, code) = take_all_staged(signer::address_of(owner));
        object_code_deployment::upgrade(owner, metadata_serialized, code, code_object);
    }

    /// Removes the `StagingArea` resource and everything staged in it, if there is anything.
    public entry fun cleanup_staging_area(owner: &signer) acquires StagingArea {
        let owner_address = signer::address_of(owner);
        if (exists<StagingArea>(owner_address)) {
            let StagingArea { chunks, keys } = move_from<StagingArea>(owner_address);
            keys.for_each(|key| {
                chunks.remove(key);
            });
            chunks.destroy_empty();
        }
    }

    fun stage_chunks_internal(
        owner: &signer,
        part_indices: vector<u16>,
        chunk_indices: vector<u16>,
        chunks: vector<vector<u8>>
    ) acquires StagingArea {
        assert!(
            part_indices.length() == chunks.length()
                && chunk_indices.length() == chunks.length(),
            error::invalid_argument(ECODE_MISMATCH)
        );
        let staging_area = borrow_or_create_staging_area(owner);
        // Every chunk carries its position, so the order in which they are staged doesn't matter.
        while (!chunks.is_empty()) {
            let key =
                chunk_key(
                    (part_indices.pop_back() as u64),
                    (chunk_indices.pop_back() as u64)
                );
            staging_area.stage_chunk(key, chunks.pop_back());
        };
    }

    fun stage_code_chunk_internal(
        owner: &signer,
        metadata_chunk: vector<u8>,
        code_indices: vector<u16>,
        code_chunks: vector<vector<u8>>
    ) acquires StagingArea {
        assert!(
            code_indices.length() == code_chunks.length(),
            error::invalid_argument(ECODE_MISMATCH)
        );
        let staging_area = borrow_or_create_staging_area(owner);
        if (!metadata_chunk.is_empty()) {
            staging_area.append_chunk(METADATA_PART, metadata_chunk);
        };
        code_indices.zip(
            code_chunks,
            |module_index, code_chunk| {
                staging_area.append_chunk((module_index as u64) + 1, code_chunk);
            }
        );
    }

    inline fun borrow_or_create_staging_area(owner: &signer): &mut StagingArea {
        let owner_address = signer::address_of(owner);
        if (!exists<StagingArea>(owner_address)) {
            move_to(
                owner,
                StagingArea {
                    chunks: table_with_length::new(),
                    keys: vector[]
                }
            );
        };
        borrow_global_mut<StagingArea>(owner_address)
    }

    /// Stages `chunk` after the chunks already staged for `part`.
    fun append_chunk(self: &mut StagingArea, part: u64, chunk: vector<u8>) {
        let index = 0;
        self.keys.for_each_ref(
            |key| {
                if (part_of(*key) == part) {
                    index += 1
                }
            }
        );
        self.stage_chunk(chunk_key(part, index), chunk);
    }

    fun stage_chunk(self: &mut StagingArea, key: u64, chunk: vector<u8>) {
        if (self.chunks.contains(key)) {
            // Staging the same chunk again, e.g. in a retried transaction, changes nothing.
            assert!(
                self.chunks.borrow(key) == &chunk,
                error::already_exists(ECHUNK_CONFLICT)
            );
        } else {
            self.chunks.add(key, chunk);
            self.keys.push_back(key);
        }
    }

    /// Assembles the package from everything that is staged, for the `stage_code_chunk*` functions.
    fun take_all_staged(
        owner_address: address
    ): (vector<u8>, vector<vector<u8>>) acquires StagingArea {
        let chunk_counts = vector[];
        borrow_global<StagingArea>(owner_address).keys.for_each_ref(
            |key| {
                let part = part_of(*key);
                while (chunk_counts.length() <= part) {
                    chunk_counts.push_back(0);
                };
                chunk_counts[part] += 1;
            }
        );
        take_staged_package(owner_address, chunk_counts)
    }

    /// Removes the staging area and assembles the package from it: its metadata and the code of
    /// each module. `chunk_counts[p]` is the number of chunks in part `p`; exactly those chunks
    /// have to be staged.
    fun take_staged_package(
        owner_address: address, chunk_counts: vector<u16>
    ): (vector<u8>, vector<vector<u8>>) acquires StagingArea {
        // A package has metadata and at least one module.
        assert!(chunk_counts.length() > 1, error::invalid_state(EMISSING_CHUNK));
        let StagingArea { chunks, keys: _ } = move_from<StagingArea>(owner_address);
        let metadata_serialized = take_part(&mut chunks, METADATA_PART, chunk_counts[0]);
        let code = vector[];
        for (part in 1..chunk_counts.length()) {
            code.push_back(take_part(&mut chunks, part, chunk_counts[part]));
        };
        // Every staged chunk went into the package, so nothing left from another publish ends up in it.
        assert!(chunks.length() == 0, error::invalid_state(EUNEXPECTED_CHUNKS));
        chunks.destroy_empty();
        (metadata_serialized, code)
    }

    /// Removes the chunks of `part` and returns them concatenated.
    fun take_part(
        chunks: &mut TableWithLength<u64, vector<u8>>, part: u64, num_chunks: u16
    ): vector<u8> {
        assert!(num_chunks > 0, error::invalid_state(EMISSING_CHUNK));
        let bytes = take_chunk(chunks, chunk_key(part, 0));
        for (index in 1..(num_chunks as u64)) {
            bytes.append(take_chunk(chunks, chunk_key(part, index)));
        };
        bytes
    }

    fun take_chunk(chunks: &mut TableWithLength<u64, vector<u8>>, key: u64): vector<u8> {
        assert!(chunks.contains(key), error::invalid_state(EMISSING_CHUNK));
        chunks.remove(key)
    }

    fun chunk_key(part: u64, index: u64): u64 {
        (part << 32) | index
    }

    fun part_of(key: u64): u64 {
        key >> 32
    }

    #[test(owner = @0x123)]
    fun test_chunks_staged_in_any_order(owner: &signer) acquires StagingArea {
        // Module 1 and the second metadata chunk, then module 0 and the first metadata chunk.
        stage_chunks(
            owner,
            vector[2, 0],
            vector[0, 1],
            vector[x"cc", x"bb"]
        );
        stage_chunks(
            owner,
            vector[1, 0],
            vector[0, 0],
            vector[x"dd", x"aa"]
        );

        let (metadata, code) =
            take_staged_package(signer::address_of(owner), vector[2, 1, 1]);
        assert!(metadata == x"aabb");
        assert!(code == vector[x"dd", x"cc"]);
        assert!(
            !exists<StagingArea>(signer::address_of(owner))
        );
    }

    #[test(owner = @0x123)]
    fun test_repeated_chunk_is_ignored(owner: &signer) acquires StagingArea {
        stage_chunks(
            owner,
            vector[0, 1],
            vector[0, 0],
            vector[x"aa", x"bb"]
        );
        // A retried transaction stages the same chunks again.
        stage_chunks(
            owner,
            vector[0, 1],
            vector[0, 0],
            vector[x"aa", x"bb"]
        );

        let (metadata, code) = take_staged_package(
            signer::address_of(owner), vector[1, 1]
        );
        assert!(metadata == x"aa");
        assert!(code == vector[x"bb"]);
    }

    #[test(owner = @0x123)]
    #[expected_failure(abort_code = 0x80004, location = Self)]
    fun test_conflicting_chunk_fails(owner: &signer) acquires StagingArea {
        stage_chunks(owner, vector[1], vector[0], vector[x"aa"]);
        stage_chunks(owner, vector[1], vector[0], vector[x"bb"]);
    }

    #[test(owner = @0x123)]
    #[expected_failure(abort_code = 0x10001, location = Self)]
    fun test_stage_chunks_with_mismatched_lengths_fails(owner: &signer) acquires StagingArea {
        stage_chunks(
            owner,
            vector[0, 1],
            vector[0],
            vector[x"aa", x"bb"]
        );
    }

    #[test(owner = @0x123)]
    #[expected_failure(abort_code = 0x30003, location = Self)]
    fun test_publish_with_missing_chunk_fails(owner: &signer) acquires StagingArea {
        // Module 0 has two chunks, but only its second one is staged.
        stage_chunks(
            owner,
            vector[0, 1],
            vector[0, 1],
            vector[x"aa", x"bb"]
        );
        stage_chunks_and_publish_to_account(
            owner,
            vector[],
            vector[],
            vector[],
            vector[1, 2]
        );
    }

    #[test(owner = @0x123)]
    #[expected_failure(abort_code = 0x30005, location = Self)]
    fun test_publish_with_unexpected_chunk_fails(owner: &signer) acquires StagingArea {
        // A chunk of a second module, which the package doesn't have, e.g. left from another publish.
        stage_chunks(
            owner,
            vector[0, 1, 2],
            vector[0, 0, 0],
            vector[x"aa", x"bb", x"cc"]
        );
        stage_chunks_and_publish_to_account(
            owner,
            vector[],
            vector[],
            vector[],
            vector[1, 1]
        );
    }

    #[test(owner = @0x123)]
    fun test_stage_code_chunk_appends_in_order(owner: &signer) acquires StagingArea {
        stage_code_chunk(
            owner,
            x"0102",
            vector[0, 1],
            vector[x"aa", x"bb"]
        );
        // Chunks for parts that already have chunks are appended to them.
        stage_code_chunk(owner, x"03", vector[0], vector[x"cc"]);

        let (metadata, code) = take_all_staged(signer::address_of(owner));
        assert!(metadata == x"010203");
        assert!(code == vector[x"aacc", x"bb"]);
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

    #[test(owner = @0x123)]
    fun test_cleanup(owner: &signer) acquires StagingArea {
        // With nothing staged, there is nothing to do.
        cleanup_staging_area(owner);

        stage_chunks(
            owner,
            vector[0, 3],
            vector[5, 2],
            vector[x"aa", x"bb"]
        );
        stage_code_chunk(owner, x"01", vector[0], vector[x"cc"]);
        cleanup_staging_area(owner);
        assert!(
            !exists<StagingArea>(signer::address_of(owner))
        );
    }
}
