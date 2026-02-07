/// # Aptos Large Packages Framework v2
///
/// This module provides a framework for uploading large packages to the Aptos network with
/// separate uploader and publisher roles. One user can upload chunks and another can publish.
///
/// ## Key Features
/// - **Separated Roles**: Allows one address to upload chunks and another to publish
/// - **Proposal System**: Uses unique proposal IDs to track staged packages
/// - **Flexible Publishing**: Supports account, object, and object upgrade publishing
///
/// ## Workflow
/// 1. **Uploader stages chunks**: Call `stage_code_chunk` multiple times with proposal ID
/// 2. **Publisher completes**: Call one of the publish functions to deploy the package
/// 3. **Cleanup**: Either party can cleanup canceled proposals
///
/// ## Security Notes
/// - Only the designated publisher can complete a proposal
/// - Proposals are tightly coupled to both uploader and publisher addresses
/// - Proposals can be canceled by either the uploader or publisher
module aptos_experimental::large_packages_v2 {
    use std::error;
    use std::signer;
    use std::vector;
    use aptos_std::table::{Self, Table};

    use aptos_framework::code::{Self, PackageRegistry};
    use aptos_framework::object::{Object};
    use aptos_framework::object_code_deployment;

    /// Code indices and code chunks should be the same length
    const ECODE_MISMATCH: u64 = 1;
    /// Only the designated publisher can publish this proposal
    const ENOT_AUTHORIZED_PUBLISHER: u64 = 2;
    /// Proposal does not exist
    const EPROPOSAL_NOT_FOUND: u64 = 3;
    /// Only uploader or publisher can cleanup proposal
    const ENOT_AUTHORIZED_CLEANUP: u64 = 4;

    /// Global staging area that stores all proposals
    struct StagingArea has key {
        proposals: Table<ProposalKey, ProposalData>,
    }

    /// Unique key for each proposal
    struct ProposalKey has copy, drop, store {
        uploader: address,
        publisher: address,
        proposal_id: u64
    }

    /// Data for a staged package proposal
    struct ProposalData has store {
        metadata_serialized: vector<u8>,
        code: vector<vector<u8>>,
    }

    /// Initialize the module with global staging area
    fun init_module(framework: &signer) {
        move_to(framework, StagingArea {
            proposals: table::new<ProposalKey, ProposalData>(),
        });
    }

    /// Stage code chunks for a proposal (can be called by any uploader)
    public entry fun stage_code_chunk(
        uploader: &signer,
        publisher: address,
        proposal_id: u64,
        metadata_chunk: vector<u8>,
        code_indices: vector<u16>,
        code_chunks: vector<vector<u8>>
    ) acquires StagingArea {
        stage_code_chunk_internal(
            uploader,
            publisher,
            proposal_id,
            metadata_chunk,
            code_indices,
            code_chunks
        );
    }

    /// Publisher publishes the staged package to their account
    public entry fun publish_to_account(
        publisher: &signer,
        uploader: address,
        proposal_id: u64
    ) acquires StagingArea {
        let proposal_data = remove_proposal_as_publisher(publisher, uploader, proposal_id);
        let (metadata_serialized, code) = destroy_proposal_data(proposal_data);
        code::publish_package_txn(publisher, metadata_serialized, code);
    }

    /// Publisher publishes the staged package to a new object
    public entry fun publish_to_object(
        publisher: &signer,
        uploader: address,
        proposal_id: u64
    ) acquires StagingArea {
        let proposal_data = remove_proposal_as_publisher(publisher, uploader, proposal_id);
        let (metadata_serialized, code) = destroy_proposal_data(proposal_data);
        object_code_deployment::publish(
            publisher,
            metadata_serialized,
            code
        );
    }

    /// Publisher upgrades an existing object code
    public entry fun upgrade_object_code(
        publisher: &signer,
        uploader: address,
        proposal_id: u64,
        code_object: Object<PackageRegistry>
    ) acquires StagingArea {
        let proposal_data = remove_proposal_as_publisher(publisher, uploader, proposal_id);
        let (metadata_serialized, code) = destroy_proposal_data(proposal_data);
        object_code_deployment::upgrade(
            publisher,
            metadata_serialized,
            code,
            code_object
        );
    }

    /// Cancel and cleanup a proposal (can be called by uploader or publisher)
    public entry fun cleanup_proposal(
        caller: &signer,
        uploader: address,
        publisher: address,
        proposal_id: u64
    ) acquires StagingArea {
        let caller_addr = signer::address_of(caller);
        assert!(
            caller_addr == uploader || caller_addr == publisher,
            error::permission_denied(ENOT_AUTHORIZED_CLEANUP)
        );

        let staging_area = &mut StagingArea[@aptos_experimental];
        let key = ProposalKey { uploader, publisher, proposal_id };

        if (staging_area.proposals.contains(key)) {
            let proposal_data = staging_area.proposals.remove(key);
            destroy_proposal_data(proposal_data);
        };
    }

    /// Internal function to stage code chunks
    inline fun stage_code_chunk_internal(
        uploader: &signer,
        publisher: address,
        proposal_id: u64,
        metadata_chunk: vector<u8>,
        code_indices: vector<u16>,
        code_chunks: vector<vector<u8>>
    ) {
        assert!(
            code_indices.length() == code_chunks.length(),
            error::invalid_argument(ECODE_MISMATCH)
        );

        let uploader_addr = signer::address_of(uploader);
        let staging_area = &mut StagingArea[@aptos_experimental];
        let key = ProposalKey { uploader: uploader_addr, publisher, proposal_id };

        // Create new proposal if it doesn't exist
        if (!staging_area.proposals.contains(key)) {
            staging_area.proposals.add(key, ProposalData {
                metadata_serialized: vector::empty(),
                code: vector[],
            });
        };

        let proposal_data = staging_area.proposals.borrow_mut(key);

        // Append metadata if provided
        if (!metadata_chunk.is_empty()) {
            proposal_data.metadata_serialized.append(metadata_chunk);
        };

        // Add or append code chunks
        let code_chunks_len = code_chunks.length();
        for (i in 0..code_chunks_len) {
            let inner_code = code_chunks[i];
            let idx = (code_indices[i] as u64);

            // Ensure the vector is large enough
            while (proposal_data.code.length() <= idx) {
                proposal_data.code.push_back(vector[]);
            };

            // Append to the code at the given index
            proposal_data.code[idx].append(inner_code);
        };
    }

    /// Remove a proposal as the publisher
    inline fun remove_proposal_as_publisher(
        publisher: &signer,
        uploader: address,
        proposal_id: u64
    ): ProposalData {
        let publisher_addr = signer::address_of(publisher);
        let staging_area = &mut StagingArea[@aptos_experimental];
        let key = ProposalKey { uploader, publisher: publisher_addr, proposal_id };

        assert!(
            staging_area.proposals.contains(key),
            error::not_found(EPROPOSAL_NOT_FOUND)
        );

        staging_area.proposals.remove(key)
    }

    /// Assemble the module code from chunks
    inline fun assemble_module_code(proposal_data: &ProposalData): vector<vector<u8>> {
        proposal_data.code
    }

    /// Destroy proposal data and clean up resources
    inline fun destroy_proposal_data(proposal_data: ProposalData): (vector<u8>, vector<vector<u8>>) {
        let ProposalData {
            metadata_serialized,
            code,
        } = proposal_data;
        (metadata_serialized, code)
    }

    /// Check if a proposal exists
    #[view]
    public fun proposal_exists(
        uploader: address,
        publisher: address,
        proposal_id: u64
    ): bool acquires StagingArea {
        let staging_area = &StagingArea[@aptos_experimental];
        let key = ProposalKey { uploader, publisher, proposal_id };
        staging_area.proposals.contains(key)
    }

    /// Get the metadata size for a proposal
    #[view]
    public fun get_proposal_metadata_size(
        uploader: address,
        publisher: address,
        proposal_id: u64
    ): u64 acquires StagingArea {
        let staging_area = &StagingArea[@aptos_experimental];
        let key = ProposalKey { uploader, publisher, proposal_id };

        if (staging_area.proposals.contains(key)) {
            let proposal_data = staging_area.proposals.borrow(key);
            proposal_data.metadata_serialized.length()
        } else {
            0
        }
    }

    /// Get the number of modules in a proposal
    #[view]
    public fun get_proposal_module_count(
        uploader: address,
        publisher: address,
        proposal_id: u64
    ): u64 acquires StagingArea {
        let staging_area = &StagingArea[@aptos_experimental];
        let key = ProposalKey { uploader, publisher, proposal_id };

        if (staging_area.proposals.contains(key)) {
            let proposal_data = staging_area.proposals.borrow(key);
            proposal_data.code.length() + 1
        } else {
            0
        }
    }

    /// Get the size of a specific module in a proposal
    #[view]
    public fun get_proposal_module_size(
        uploader: address,
        publisher: address,
        proposal_id: u64,
        module_index: u64
    ): u64 acquires StagingArea {
        let staging_area = &StagingArea[@aptos_experimental];
        let key = ProposalKey { uploader, publisher, proposal_id };

        if (!staging_area.proposals.contains(key)) {
            return 0
        };

        let proposal_data = staging_area.proposals.borrow(key);
        if (module_index >= proposal_data.code.length()) {
            return 0
        };

        proposal_data.code[module_index].length()
    }

    /// Get the total size of all code in a proposal
    #[view]
    public fun get_proposal_total_code_size(
        uploader: address,
        publisher: address,
        proposal_id: u64
    ): u64 acquires StagingArea {
        let staging_area = &StagingArea[@aptos_experimental];
        let key = ProposalKey { uploader, publisher, proposal_id };

        if (!staging_area.proposals.contains(key)) {
            return 0
        };

        let proposal_data = staging_area.proposals.borrow(key);
        let total_size = 0u64;
        let code_len = proposal_data.code.length();

        for (i in 0..code_len) {
            total_size += proposal_data.code[i].length();
        };

        total_size
    }

    /// Get proposal summary information
    #[view]
    public fun get_proposal_summary(
        uploader: address,
        publisher: address,
        proposal_id: u64
    ): (bool, u64, u64, u64) acquires StagingArea {
        let exists = proposal_exists(uploader, publisher, proposal_id);
        if (!exists) {
            return (false, 0, 0, 0)
        };

        let metadata_size = get_proposal_metadata_size(uploader, publisher, proposal_id);
        let module_count = get_proposal_module_count(uploader, publisher, proposal_id);
        let total_code_size = get_proposal_total_code_size(uploader, publisher, proposal_id);

        (exists, metadata_size, module_count, total_code_size)
    }

    #[test_only]
    public fun init_module_for_test(framework: &signer) {
        init_module(framework);
    }
}
