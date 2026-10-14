/// # Aptos Large Packages Framework
///
/// A transaction is limited in size, and a package with its metadata is often bigger than that. This
/// module lets an account upload a package in chunks, over as many transactions as it takes, and then
/// publish it with a final call: to the account, or to an object.
///
/// The Aptos CLI implements this flow with the `--chunked-publish` flag:
/// - `aptos move publish [OPTIONS] --chunked-publish`
/// - `aptos move create-object-and-publish-package [OPTIONS] --address-name <ADDRESS_NAME> --chunked-publish`
/// - `aptos move upgrade-object-package [OPTIONS] --object-address <OBJECT_ADDRESS> --chunked-publish`
///
/// # Parts, chunks and positions
///
/// A package is made of *parts*: part 0 is its serialized `PackageMetadata`, and part `i + 1` is the
/// code of module `i`, in the order in which the modules are published. Each part is split into
/// *chunks*, which are numbered from 0 in the order of their bytes. A chunk is identified by its
/// *position*: the index of its part, and its index in the part.
///
/// For example, a package with 120 bytes of metadata and two modules of 100 and 40 bytes, split into
/// chunks of at most 64 bytes, has five chunks:
///
/// | part                | chunks (position: size)                |
/// |---------------------|----------------------------------------|
/// | 0 (metadata)        | (0, 0): 64 bytes, (0, 1): 56 bytes     |
/// | 1 (first module)    | (1, 0): 64 bytes, (1, 1): 36 bytes     |
/// | 2 (second module)   | (2, 0): 40 bytes                       |
///
/// and the number of chunks in each part is `[2, 2, 1]`.
///
/// # Staging
///
/// `stage_chunks(owner, part_indices, chunk_indices, chunks)` stages `chunks[i]` at the position
/// `(part_indices[i], chunk_indices[i])`, and the three vectors have to have the same length. Every
/// chunk carries its position, so:
/// - chunks can be staged in any order, over any number of transactions;
/// - staging a chunk that is already staged, with the same bytes, changes nothing, so a transaction
///   can be retried;
/// - staging other bytes at a position that is taken aborts with `ECHUNK_CONFLICT`, which catches
///   chunks left from a different package.
///
/// The chunks are kept in the `StagingArea` resource of the account that stages them, each in a
/// storage slot of its own, so staging a chunk doesn't read or write the chunks that are staged
/// already. Only the account itself can stage chunks in its `StagingArea`.
///
/// # Publishing
///
/// `stage_chunks_and_publish_to_account`, `stage_chunks_and_publish_to_object` and
/// `stage_chunks_and_upgrade_object_code` stage the chunks they are given, like `stage_chunks`, and
/// then publish the package that is staged. Their `chunk_counts` argument is the number of chunks in
/// each part (`[2, 2, 1]` for the package above), which is what tells the module which chunks make
/// up the package. Publishing:
/// - aborts with `EMISSING_CHUNK` if a chunk of the package is not staged;
/// - aborts with `EUNEXPECTED_CHUNKS` if other chunks are staged, e.g. left from another package, so
///   that they can't end up in the package;
/// - removes the `StagingArea`, which refunds the storage fee that was paid for the chunks;
/// - hands the metadata and the code to the framework, which checks them as it does for any publish:
///   an invalid package, or one that can't upgrade the published package, fails there.
///
/// A transaction that fails has no effect, so when the one that publishes fails, the chunks that
/// earlier transactions staged stay staged.
///
/// # Resuming and cleaning up
///
/// What is staged is readable on chain: `StagingArea.keys` has the key of every staged chunk, and
/// `StagingArea.chunks` has its bytes. A client can use that to resume a publish that stopped, by
/// staging only the chunks that are missing; the Aptos CLI does that when the staged chunks are
/// identical to those of the package.
///
/// An account has one `StagingArea`, so a package has to be published, or the staging area cleaned
/// up, before another package is staged. `cleanup_staging_area` removes the `StagingArea` with
/// everything staged in it, and does nothing if there is none.
///
/// # Earlier versions of this module
///
/// The `stage_code_chunk*` functions are those of earlier versions of this module, and are kept for
/// existing clients, such as older versions of the Aptos CLI. Their chunks carry no positions: each
/// chunk is appended to the chunks that are already staged for its part. So the chunks of a part have
/// to be staged in order, a chunk that is staged twice can't be told from two chunks, and the package
/// that is published is made of everything that is staged. Prefer the `stage_chunks*` functions.
///
/// # Errors
///
/// | constant             | abort code | raised when                                               |
/// |----------------------|------------|-----------------------------------------------------------|
/// | `ECODE_MISMATCH`     | `0x10001`  | the vectors that describe the chunks differ in length     |
/// | `EMISSING_CHUNK`     | `0x30003`  | a chunk that the package needs is not staged              |
/// | `ECHUNK_CONFLICT`    | `0x80004`  | different bytes are staged at the position of a chunk     |
/// | `EUNEXPECTED_CHUNKS` | `0x30005`  | chunks that the package doesn't have are staged           |
///
/// The framework's own aborts are not listed here, e.g. `object_code_deployment::upgrade` aborts if
/// the account doesn't own the code object.
///
/// # Notes
///
/// * This module is released at `0x7` (aptos-experimental) only through genesis, so a network has
///   the version that was current when it was created. Mainnet has none at `0x7`, but has an earlier
///   version, with only the `stage_code_chunk*` functions, at
///   `0x0e1ca3011bdd07246d4d16d909dbb2d6953a86c4735d5acf5865d962c630cce7`.
/// * The layout of `StagingArea`, and the key of a chunk (`chunk_key`), are read by clients, so they
///   are part of the interface of the module.
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

    #[test_only]
    use std::bcs;

    #[test_only]
    use std::string;

    #[test_only]
    use aptos_framework::account;

    #[test_only]
    use aptos_framework::event;

    #[test_only]
    use aptos_framework::object;

    // The errors are raised with `error::invalid_argument`, `error::invalid_state` and
    // `error::already_exists`, so the abort codes are 0x10001, 0x30003, 0x80004 and 0x30005, as in
    // the table in the documentation of the module. The documentation of a constant is the
    // description that users see when a transaction aborts with it, so it is kept short.

    /// The vectors that describe the chunks to stage have different lengths.
    const ECODE_MISMATCH: u64 = 1;
    /// A chunk that the package needs is not staged.
    const EMISSING_CHUNK: u64 = 3;
    /// Other bytes are already staged at the position of a chunk.
    const ECHUNK_CONFLICT: u64 = 4;
    /// Chunks are staged that the package being published doesn't have.
    const EUNEXPECTED_CHUNKS: u64 = 5;

    /// The part of a package that is its metadata. The code of module `i` is part `i + 1`.
    const METADATA_PART: u64 = 0;

    /// The chunks that an account has staged for a package that it is going to publish.
    ///
    /// An account has at most one. It is created when the account stages its first chunk, and removed
    /// when the account publishes the package, or calls `cleanup_staging_area`.
    ///
    /// Invariant: `keys` has the key of every chunk in `chunks`, once. The layout is read by clients,
    /// e.g. to resume a publish, so it is part of the interface of the module.
    struct StagingArea has key {
        /// The staged chunks, each in a storage slot of its own, keyed by `chunk_key(part, index)`.
        chunks: TableWithLength<u64, vector<u8>>,
        /// The key of every staged chunk, so that they can be removed without knowing the package.
        keys: vector<u64>
    }

    /// Stages chunks of a package: `chunks[i]` is chunk `chunk_indices[i]` of part `part_indices[i]`.
    ///
    /// Chunks can be staged in any order, over any number of transactions. A chunk that is already
    /// staged, with the same bytes, is left as it is.
    ///
    /// # Aborts
    /// * `ECODE_MISMATCH` if `part_indices`, `chunk_indices` and `chunks` have different lengths.
    /// * `ECHUNK_CONFLICT` if other bytes are staged at the position of one of the chunks.
    public entry fun stage_chunks(
        owner: &signer,
        part_indices: vector<u16>,
        chunk_indices: vector<u16>,
        chunks: vector<vector<u8>>
    ) acquires StagingArea {
        stage_chunks_internal(owner, part_indices, chunk_indices, chunks);
    }

    /// Stages the given chunks, like `stage_chunks`, then publishes the package that is staged under
    /// `owner`, which removes the staging area. `chunk_counts[p]` is the number of chunks in part
    /// `p`: the metadata first, then each module.
    ///
    /// # Aborts
    /// * `ECODE_MISMATCH` and `ECHUNK_CONFLICT`, as in `stage_chunks`.
    /// * `EMISSING_CHUNK` if `chunk_counts` doesn't have a part for the metadata and one for a
    ///   module, if a part has no chunks, or if a chunk of the package is not staged.
    /// * `EUNEXPECTED_CHUNKS` if chunks that the package doesn't have are staged.
    /// * What `code::publish_package_txn` aborts with, e.g. if the metadata is not valid.
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

    /// Stages the given chunks, like `stage_chunks`, then publishes the package that is staged to a
    /// new object, which `owner` owns. The address of the object is derived from the sequence number
    /// of the transaction (see `object_code_deployment`). `chunk_counts[p]` is the number of chunks
    /// in part `p`: the metadata first, then each module.
    ///
    /// # Aborts
    /// * As `stage_chunks_and_publish_to_account`, with what `object_code_deployment::publish`
    ///   aborts with instead of what `code::publish_package_txn` aborts with.
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

    /// Stages the given chunks, like `stage_chunks`, then upgrades the package in `code_object` to the
    /// package that is staged. `chunk_counts[p]` is the number of chunks in part `p`: the metadata
    /// first, then each module.
    ///
    /// # Aborts
    /// * As `stage_chunks_and_publish_to_account`, with what `object_code_deployment::upgrade`
    ///   aborts with instead of what `code::publish_package_txn` aborts with, e.g. if `owner`
    ///   doesn't own `code_object`.
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

    /// Stages chunks in order, as earlier versions of this module did: `metadata_chunk`, if it isn't
    /// empty, is appended to the metadata, and `code_chunks[i]` to the code of module
    /// `code_indices[i]`. Each chunk goes after the chunks already staged for its part, so the chunks
    /// of a part have to be staged in order. Prefer `stage_chunks`, whose chunks carry positions.
    ///
    /// # Aborts
    /// * `ECODE_MISMATCH` if `code_indices` and `code_chunks` have different lengths.
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

    /// Stages chunks like `stage_code_chunk`, then publishes the package that is made of everything
    /// that is staged under `owner`, which removes the staging area.
    ///
    /// # Aborts
    /// * `ECODE_MISMATCH`, as in `stage_code_chunk`.
    /// * `EMISSING_CHUNK` if no module is staged, or if a module index below the highest one that is
    ///   staged was never staged.
    /// * What `code::publish_package_txn` aborts with, e.g. if the metadata is not valid.
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

    /// Stages chunks like `stage_code_chunk`, then publishes the package that is made of everything
    /// that is staged to a new object, which `owner` owns.
    ///
    /// # Aborts
    /// * As `stage_code_chunk_and_publish_to_account`, with what `object_code_deployment::publish`
    ///   aborts with instead of what `code::publish_package_txn` aborts with.
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

    /// Stages chunks like `stage_code_chunk`, then upgrades the package in `code_object` to the
    /// package that is made of everything that is staged.
    ///
    /// # Aborts
    /// * As `stage_code_chunk_and_publish_to_account`, with what `object_code_deployment::upgrade`
    ///   aborts with instead of what `code::publish_package_txn` aborts with, e.g. if `owner`
    ///   doesn't own `code_object`.
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

    /// Removes the `StagingArea` of `owner`, with everything staged in it, which refunds the storage
    /// fee that was paid for it. Does nothing if `owner` has no `StagingArea`.
    ///
    /// Use it to start over, e.g. after staging a package that is not going to be published.
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

    /// Stages the chunks of `stage_chunks`, and creates the staging area if there is none.
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

    /// Stages the chunks of `stage_code_chunk` in order, and creates the staging area if there is
    /// none.
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
        // Consume the chunks in their original order, instead of copying them.
        code_indices.reverse();
        code_chunks.reverse();
        while (!code_chunks.is_empty()) {
            let module_index = code_indices.pop_back();
            staging_area.append_chunk((module_index as u64) + 1, code_chunks.pop_back());
        };
    }

    /// The staging area of `owner`, which is created if there is none. It is inline because a
    /// function can't return a reference into global storage.
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

    /// Stages `chunk` after the chunks already staged for `part`: its index is their number. This is
    /// for the `stage_code_chunk*` functions, whose chunks have no positions.
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

    /// Stages `chunk` under `key`. If a chunk is staged under `key` already, it has to have the same
    /// bytes, and nothing changes: that is what makes it harmless to stage a chunk again.
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
    /// A part that has no chunks, e.g. a module index that was skipped, gets a chunk count of 0, so
    /// that `take_staged_package` aborts.
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
    ///
    /// Aborts with `EMISSING_CHUNK` if `chunk_counts` doesn't have a part for the metadata and one
    /// for a module, if a part has no chunks, or if a chunk is not staged; and with
    /// `EUNEXPECTED_CHUNKS` if chunks are staged that are not part of the package.
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

    /// Removes the chunk that is staged under `key`, which has to be staged.
    fun take_chunk(chunks: &mut TableWithLength<u64, vector<u8>>, key: u64): vector<u8> {
        assert!(chunks.contains(key), error::invalid_state(EMISSING_CHUNK));
        chunks.remove(key)
    }

    /// The key under which the chunk at the position `(part, index)` is staged: the part is in the
    /// upper 32 bits and the index in the lower ones. Clients rely on this layout.
    fun chunk_key(part: u64, index: u64): u64 {
        (part << 32) | index
    }

    /// The part of the chunk that is staged under `key`.
    fun part_of(key: u64): u64 {
        key >> 32
    }

    // Tests
    //
    // They cover every function of the module and every branch in it, except for the return of
    // `stage_chunks_and_upgrade_object_code` and `stage_code_chunk_and_upgrade_object_code` after
    // the framework has upgraded the code object: see the comment before the tests of upgrades.
    // The end-to-end tests of publishing large packages
    // (aptos-move/e2e-move-tests/src/tests/large_package_publishing.rs) go through the VM, and cover
    // publishes and upgrades that succeed.
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

    #[test(owner = @0x123)]
    fun test_stage_chunks_without_chunks_creates_an_empty_staging_area(
        owner: &signer
    ) acquires StagingArea {
        let owner_address = signer::address_of(owner);
        stage_chunks(owner, vector[], vector[], vector[]);

        let staging_area = borrow_global<StagingArea>(owner_address);
        assert!(staging_area.keys.is_empty());
        assert!(staging_area.chunks.length() == 0);
        cleanup_staging_area(owner);
        assert!(!exists<StagingArea>(owner_address));
    }

    #[test]
    fun test_chunk_key_layout() {
        // The part is in the upper 32 bits and the index in the lower ones. The Aptos CLI reads the
        // keys of staged chunks, so this layout is part of the interface of the module.
        assert!(chunk_key(0, 0) == 0);
        assert!(chunk_key(0, 5) == 5);
        assert!(chunk_key(1, 0) == 0x100000000);
        assert!(chunk_key(65535, 65535) == 0xFFFF0000FFFF);
        assert!(part_of(chunk_key(7, 300)) == 7);
    }

    #[test(owner = @0x123)]
    #[expected_failure(abort_code = 0x10001, location = Self)]
    fun test_stage_chunks_with_mismatched_part_indices_fails(
        owner: &signer
    ) acquires StagingArea {
        stage_chunks(
            owner,
            vector[0],
            vector[0, 1],
            vector[x"aa", x"bb"]
        );
    }

    #[test(owner = @0x123)]
    #[expected_failure(abort_code = 0x10001, location = Self)]
    fun test_stage_code_chunk_with_mismatched_lengths_fails(
        owner: &signer
    ) acquires StagingArea {
        stage_code_chunk(owner, x"", vector[0, 1], vector[x"aa"]);
    }

    #[test(owner = @0x123)]
    #[expected_failure(abort_code = 0x30003, location = Self)]
    fun test_publish_without_modules_fails(owner: &signer) acquires StagingArea {
        // A package has metadata and at least one module.
        stage_chunks(owner, vector[0], vector[0], vector[x"aa"]);
        stage_chunks_and_publish_to_account(owner, vector[], vector[], vector[], vector[1]);
    }

    #[test(owner = @0x123)]
    #[expected_failure(abort_code = 0x30003, location = Self)]
    fun test_publish_with_zero_chunk_count_fails(owner: &signer) acquires StagingArea {
        // The package says that its module has no chunks.
        stage_chunks(owner, vector[0], vector[0], vector[x"aa"]);
        stage_chunks_and_publish_to_account(
            owner,
            vector[],
            vector[],
            vector[],
            vector[1, 0]
        );
    }

    // The tests that publish need serialized package metadata, which has the layout of
    // `aptos_framework::code::PackageMetadata`. Its fields can't be set outside that module, so
    // these structs have the same fields.
    #[test_only]
    struct TestUpgradePolicy has copy, drop {
        policy: u8
    }

    #[test_only]
    struct TestModuleMetadata has copy, drop {
        name: string::String,
        source: vector<u8>,
        source_map: vector<u8>,
        // An empty `Option<Any>`.
        extension: vector<u8>
    }

    #[test_only]
    struct TestPackageMetadata has copy, drop {
        name: string::String,
        upgrade_policy: TestUpgradePolicy,
        upgrade_number: u64,
        source_digest: string::String,
        manifest: vector<u8>,
        modules: vector<TestModuleMetadata>,
        // An empty `vector<PackageDep>`.
        deps: vector<u8>,
        // An empty `Option<Any>`.
        extension: vector<u8>
    }

    #[test_only]
    /// The serialized metadata of a package with a compatible upgrade policy, with this name and
    /// modules with these names.
    fun test_metadata(name: vector<u8>, module_names: vector<vector<u8>>): vector<u8> {
        bcs::to_bytes(
            &TestPackageMetadata {
                name: string::utf8(name),
                upgrade_policy: TestUpgradePolicy { policy: 1 },
                upgrade_number: 0,
                source_digest: string::utf8(b""),
                manifest: vector[],
                modules: module_names.map(
                    |module_name| TestModuleMetadata {
                        name: string::utf8(module_name),
                        source: vector[],
                        source_map: vector[],
                        extension: vector[]
                    }
                ),
                deps: vector[],
                extension: vector[]
            }
        )
    }

    #[test_only]
    /// The address of the object that `object_code_deployment::publish` creates for `owner`, which
    /// is derived from the sequence number that the owner's transaction has.
    fun next_code_object_address(owner: address): address {
        let seed = vector[];
        seed.append(bcs::to_bytes(&b"aptos_framework::object_code_deployment"));
        seed.append(
            bcs::to_bytes(&(account::get_sequence_number(owner) + 1))
        );
        object::create_object_address(&owner, seed)
    }

    #[test(owner = @0x123)]
    fun test_stage_chunks_and_publish_to_account(owner: &signer) acquires StagingArea {
        let owner_address = signer::address_of(owner);
        let metadata = test_metadata(b"package", vector[b"first", b"second"]);
        let middle = metadata.length() / 2;

        // The second chunk of the metadata, and the second chunk of the second module.
        stage_chunks(
            owner,
            vector[0, 2],
            vector[1, 1],
            vector[metadata.slice(middle, metadata.length()), x"dd"]
        );
        let staging_area = borrow_global<StagingArea>(owner_address);
        assert!(staging_area.keys.length() == 2);
        assert!(staging_area.chunks.length() == 2);

        // The transaction that publishes stages the other chunks, and one that is staged already.
        stage_chunks_and_publish_to_account(
            owner,
            vector[0, 1, 2, 2],
            vector[0, 0, 0, 1],
            vector[metadata.slice(0, middle), x"cc", x"dc", x"dd"],
            vector[2, 1, 2]
        );

        assert!(!exists<StagingArea>(owner_address));
        // The framework was asked to publish the package.
        assert!(
            event::emitted_events<code::PublishPackage>().length() == 1
        );
    }

    #[test(owner = @0x123)]
    fun test_stage_chunks_and_publish_to_object(owner: &signer) acquires StagingArea {
        let owner_address = signer::address_of(owner);
        account::create_account_for_test(owner_address);
        let metadata = test_metadata(b"package", vector[b"module"]);

        let code_object_address = next_code_object_address(owner_address);
        stage_chunks_and_publish_to_object(
            owner,
            vector[0, 1],
            vector[0, 0],
            vector[metadata, x"aa"],
            vector[1, 1]
        );

        assert!(!exists<StagingArea>(owner_address));
        assert!(object::object_exists<PackageRegistry>(code_object_address));
        assert!(
            event::emitted_events<code::PublishPackage>().length() == 1
        );
        let code_object = object::address_to_object<PackageRegistry>(code_object_address);
        assert!(object::is_owner(code_object, owner_address));
    }

    // A unit test is a single transaction, and the framework accepts one request to publish modules
    // per transaction. So an upgrade of a code object can't succeed after the publish that creates
    // it here, and these tests only go as far as the framework checks the owner of the code object.
    // The end-to-end tests of publishing large packages cover upgrades that succeed.
    #[test(owner = @0x123, other = @0x456)]
    #[expected_failure(abort_code = 0x50002, location = aptos_framework::object_code_deployment)]
    fun test_stage_chunks_and_upgrade_object_code_of_another_account_fails(
        owner: &signer, other: &signer
    ) acquires StagingArea {
        let owner_address = signer::address_of(owner);
        account::create_account_for_test(owner_address);
        let metadata = test_metadata(b"package", vector[b"module"]);
        let code_object_address = next_code_object_address(owner_address);
        stage_chunks_and_publish_to_object(
            owner,
            vector[0, 1],
            vector[0, 0],
            vector[metadata, x"aa"],
            vector[1, 1]
        );
        let code_object = object::address_to_object<PackageRegistry>(code_object_address);

        stage_chunks_and_upgrade_object_code(
            other,
            vector[0, 1],
            vector[0, 0],
            vector[metadata, x"bb"],
            vector[1, 1],
            code_object
        );
    }

    #[test(owner = @0x123)]
    fun test_stage_code_chunk_and_publish_to_account(owner: &signer) acquires StagingArea {
        let owner_address = signer::address_of(owner);
        let metadata = test_metadata(b"package", vector[b"first", b"second"]);
        let middle = metadata.length() / 2;

        // The chunks of the first module, and of the metadata, are appended in order.
        stage_code_chunk(
            owner,
            metadata.slice(0, middle),
            vector[0],
            vector[x"aa"]
        );
        stage_code_chunk_and_publish_to_account(
            owner,
            metadata.slice(middle, metadata.length()),
            vector[1, 0],
            vector[x"bb", x"cc"]
        );

        assert!(!exists<StagingArea>(owner_address));
        // The framework was asked to publish the package.
        assert!(
            event::emitted_events<code::PublishPackage>().length() == 1
        );
    }

    #[test(owner = @0x123)]
    fun test_stage_code_chunk_and_publish_to_object(owner: &signer) acquires StagingArea {
        let owner_address = signer::address_of(owner);
        account::create_account_for_test(owner_address);
        let metadata = test_metadata(b"package", vector[b"module"]);

        let code_object_address = next_code_object_address(owner_address);
        stage_code_chunk_and_publish_to_object(owner, metadata, vector[0], vector[x"aa"]);

        assert!(!exists<StagingArea>(owner_address));
        assert!(object::object_exists<PackageRegistry>(code_object_address));
        assert!(
            event::emitted_events<code::PublishPackage>().length() == 1
        );
        let code_object = object::address_to_object<PackageRegistry>(code_object_address);
        assert!(object::is_owner(code_object, owner_address));
    }

    #[test(owner = @0x123, other = @0x456)]
    #[expected_failure(abort_code = 0x50002, location = aptos_framework::object_code_deployment)]
    fun test_stage_code_chunk_and_upgrade_object_code_of_another_account_fails(
        owner: &signer, other: &signer
    ) acquires StagingArea {
        let owner_address = signer::address_of(owner);
        account::create_account_for_test(owner_address);
        let metadata = test_metadata(b"package", vector[b"module"]);
        let code_object_address = next_code_object_address(owner_address);
        stage_code_chunk_and_publish_to_object(owner, metadata, vector[0], vector[x"aa"]);
        let code_object = object::address_to_object<PackageRegistry>(code_object_address);

        stage_code_chunk_and_upgrade_object_code(
            other,
            metadata,
            vector[0],
            vector[x"bb"],
            code_object
        );
    }
}
