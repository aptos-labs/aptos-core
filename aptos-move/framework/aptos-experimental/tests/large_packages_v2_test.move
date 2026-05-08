#[test_only]
module aptos_experimental::large_packages_v4_test {
    use std::signer;
    use std::vector;
    use aptos_std::string_utils;
    use aptos_framework::account;
    use aptos_framework::code;
    use aptos_framework::object;
    use aptos_experimental::large_packages_v4;

    /// Test helper to create test accounts
    fun setup_test_accounts(): (signer, signer, signer) {
        let framework = account::create_account_for_test(@aptos_experimental);
        let uploader = account::create_account_for_test(@0xCAFE);
        let publisher = account::create_account_for_test(@0xBEEF);
        
        // Initialize the module
        large_packages_v4::init_module_for_test(&framework);
        
        (framework, uploader, publisher)
    }

    /// Helper to create test metadata
    fun create_test_metadata(size: u64): vector<u8> {
        let metadata = vector::empty<u8>();
        let i = 0;
        while (i < size) {
            vector::push_back(&mut metadata, (i as u8));
            i = i + 1;
        };
        metadata
    }

    /// Helper to create test code chunks
    fun create_test_code(module_count: u64, chunk_size: u64): (vector<u16>, vector<vector<u8>>) {
        let indices = vector::empty<u16>();
        let chunks = vector::empty<vector<u8>>();
        
        let i = 0;
        while (i < module_count) {
            vector::push_back(&mut indices, (i as u16));
            vector::push_back(&mut chunks, create_test_metadata(chunk_size));
            i = i + 1;
        };
        
        (indices, chunks)
    }

    #[test]
    fun test_basic_staging_and_publishing() {
        let (_framework, uploader, publisher) = setup_test_accounts();
        let proposal_id = 1;
        let uploader_addr = signer::address_of(&uploader);
        let publisher_addr = signer::address_of(&publisher);
        
        // Stage metadata
        large_packages_v4::stage_code_chunk(
            &uploader,
            publisher_addr,
            proposal_id,
            create_test_metadata(100),
            vector::empty(),
            vector::empty()
        );
        
        // Verify proposal exists
        assert!(large_packages_v4::proposal_exists(uploader_addr, publisher_addr, proposal_id), 1);
        
        // Stage code chunks
        let (indices, chunks) = create_test_code(3, 50);
        large_packages_v4::stage_code_chunk(
            &uploader,
            publisher_addr,
            proposal_id,
            vector::empty(),
            indices,
            chunks
        );
        
        // Verify sizes
        assert!(large_packages_v4::get_proposal_metadata_size(uploader_addr, publisher_addr, proposal_id) == 100, 2);
        assert!(large_packages_v4::get_proposal_module_count(uploader_addr, publisher_addr, proposal_id) == 3, 3);
        assert!(large_packages_v4::get_proposal_total_code_size(uploader_addr, publisher_addr, proposal_id) == 150, 4);
    }

    #[test]
    fun test_multiple_chunk_staging() {
        let (_framework, uploader, publisher) = setup_test_accounts();
        let proposal_id = 2;
        let uploader_addr = signer::address_of(&uploader);
        let publisher_addr = signer::address_of(&publisher);
        
        // Stage in multiple calls
        large_packages_v4::stage_code_chunk(
            &uploader,
            publisher_addr,
            proposal_id,
            create_test_metadata(50),
            vector[0],
            vector[create_test_metadata(100)]
        );
        
        // Stage more metadata and another module
        large_packages_v4::stage_code_chunk(
            &uploader,
            publisher_addr,
            proposal_id,
            create_test_metadata(25),
            vector[1],
            vector[create_test_metadata(75)]
        );
        
        // Append to existing module
        large_packages_v4::stage_code_chunk(
            &uploader,
            publisher_addr,
            proposal_id,
            vector::empty(),
            vector[0],
            vector[create_test_metadata(50)]
        );
        
        // Verify accumulated sizes
        assert!(large_packages_v4::get_proposal_metadata_size(uploader_addr, publisher_addr, proposal_id) == 75, 1);
        assert!(large_packages_v4::get_proposal_module_size(uploader_addr, publisher_addr, proposal_id, 0) == 150, 2);
        assert!(large_packages_v4::get_proposal_module_size(uploader_addr, publisher_addr, proposal_id, 1) == 75, 3);
        assert!(large_packages_v4::get_proposal_total_code_size(uploader_addr, publisher_addr, proposal_id) == 225, 4);
    }

    #[test]
    fun test_sparse_module_indices() {
        let (_framework, uploader, publisher) = setup_test_accounts();
        let proposal_id = 3;
        let uploader_addr = signer::address_of(&uploader);
        let publisher_addr = signer::address_of(&publisher);
        
        // Stage modules at non-sequential indices
        large_packages_v4::stage_code_chunk(
            &uploader,
            publisher_addr,
            proposal_id,
            vector::empty(),
            vector[0, 2, 5],
            vector[
                create_test_metadata(10),
                create_test_metadata(20),
                create_test_metadata(30)
            ]
        );
        
        // Verify module count and sizes
        assert!(large_packages_v4::get_proposal_module_count(uploader_addr, publisher_addr, proposal_id) == 6, 1);
        assert!(large_packages_v4::get_proposal_module_size(uploader_addr, publisher_addr, proposal_id, 0) == 10, 2);
        assert!(large_packages_v4::get_proposal_module_size(uploader_addr, publisher_addr, proposal_id, 1) == 0, 3);
        assert!(large_packages_v4::get_proposal_module_size(uploader_addr, publisher_addr, proposal_id, 2) == 20, 4);
        assert!(large_packages_v4::get_proposal_module_size(uploader_addr, publisher_addr, proposal_id, 5) == 30, 5);
    }

    #[test]
    #[expected_failure(abort_code = 0x10001)] // ECODE_MISMATCH
    fun test_mismatched_indices_and_chunks() {
        let (_framework, uploader, publisher) = setup_test_accounts();
        let publisher_addr = signer::address_of(&publisher);
        
        // Try to stage with mismatched counts
        large_packages_v4::stage_code_chunk(
            &uploader,
            publisher_addr,
            1,
            vector::empty(),
            vector[0, 1], // 2 indices
            vector[create_test_metadata(10)] // 1 chunk
        );
    }

    #[test]
    #[expected_failure(abort_code = 0x60003)] // EPROPOSAL_NOT_FOUND
    fun test_publish_nonexistent_proposal() {
        let (_framework, _uploader, publisher) = setup_test_accounts();
        
        // Try to publish a proposal that doesn't exist
        large_packages_v4::publish_to_account(
            &publisher,
            @0x123,
            999
        );
    }

    #[test]
    fun test_cleanup_by_uploader() {
        let (_framework, uploader, publisher) = setup_test_accounts();
        let proposal_id = 4;
        let uploader_addr = signer::address_of(&uploader);
        let publisher_addr = signer::address_of(&publisher);
        
        // Create a proposal
        large_packages_v4::stage_code_chunk(
            &uploader,
            publisher_addr,
            proposal_id,
            create_test_metadata(50),
            vector::empty(),
            vector::empty()
        );
        
        assert!(large_packages_v4::proposal_exists(uploader_addr, publisher_addr, proposal_id), 1);
        
        // Cleanup as uploader
        large_packages_v4::cleanup_proposal(
            &uploader,
            uploader_addr,
            publisher_addr,
            proposal_id
        );
        
        assert!(!large_packages_v4::proposal_exists(uploader_addr, publisher_addr, proposal_id), 2);
    }

    #[test]
    fun test_cleanup_by_publisher() {
        let (_framework, uploader, publisher) = setup_test_accounts();
        let proposal_id = 5;
        let uploader_addr = signer::address_of(&uploader);
        let publisher_addr = signer::address_of(&publisher);
        
        // Create a proposal
        large_packages_v4::stage_code_chunk(
            &uploader,
            publisher_addr,
            proposal_id,
            create_test_metadata(50),
            vector::empty(),
            vector::empty()
        );
        
        assert!(large_packages_v4::proposal_exists(uploader_addr, publisher_addr, proposal_id), 1);
        
        // Cleanup as publisher
        large_packages_v4::cleanup_proposal(
            &publisher,
            uploader_addr,
            publisher_addr,
            proposal_id
        );
        
        assert!(!large_packages_v4::proposal_exists(uploader_addr, publisher_addr, proposal_id), 2);
    }

    #[test]
    #[expected_failure(abort_code = 0x50004)] // ENOT_AUTHORIZED_CLEANUP
    fun test_cleanup_by_unauthorized() {
        let (_framework, uploader, publisher) = setup_test_accounts();
        let unauthorized = account::create_account_for_test(@0xBAD);
        let proposal_id = 6;
        let uploader_addr = signer::address_of(&uploader);
        let publisher_addr = signer::address_of(&publisher);
        
        // Create a proposal
        large_packages_v4::stage_code_chunk(
            &uploader,
            publisher_addr,
            proposal_id,
            create_test_metadata(50),
            vector::empty(),
            vector::empty()
        );
        
        // Try to cleanup as unauthorized user
        large_packages_v4::cleanup_proposal(
            &unauthorized,
            uploader_addr,
            publisher_addr,
            proposal_id
        );
    }

    #[test]
    fun test_view_functions_for_empty_proposal() {
        let (_framework, _uploader, _publisher) = setup_test_accounts();
        
        // Test view functions on non-existent proposal
        assert!(!large_packages_v4::proposal_exists(@0x1, @0x2, 999), 1);
        assert!(large_packages_v4::get_proposal_metadata_size(@0x1, @0x2, 999) == 0, 2);
        assert!(large_packages_v4::get_proposal_module_count(@0x1, @0x2, 999) == 0, 3);
        assert!(large_packages_v4::get_proposal_module_size(@0x1, @0x2, 999, 0) == 0, 4);
        assert!(large_packages_v4::get_proposal_total_code_size(@0x1, @0x2, 999) == 0, 5);
        
        let (exists, metadata_size, module_count, total_size) = 
            large_packages_v4::get_proposal_summary(@0x1, @0x2, 999);
        assert!(!exists && metadata_size == 0 && module_count == 0 && total_size == 0, 6);
    }

    #[test]
    fun test_proposal_summary() {
        let (_framework, uploader, publisher) = setup_test_accounts();
        let proposal_id = 7;
        let uploader_addr = signer::address_of(&uploader);
        let publisher_addr = signer::address_of(&publisher);
        
        // Stage some data
        large_packages_v4::stage_code_chunk(
            &uploader,
            publisher_addr,
            proposal_id,
            create_test_metadata(123),
            vector[0, 1, 2],
            vector[
                create_test_metadata(100),
                create_test_metadata(200),
                create_test_metadata(300)
            ]
        );
        
        let (exists, metadata_size, module_count, total_size) = 
            large_packages_v4::get_proposal_summary(uploader_addr, publisher_addr, proposal_id);
        
        assert!(exists, 1);
        assert!(metadata_size == 123, 2);
        assert!(module_count == 3, 3);
        assert!(total_size == 600, 4);
    }

    #[test]
    fun test_multiple_proposals() {
        let (_framework, uploader, publisher) = setup_test_accounts();
        let uploader_addr = signer::address_of(&uploader);
        let publisher_addr = signer::address_of(&publisher);
        
        // Create multiple proposals
        let i = 0;
        while (i < 5) {
            large_packages_v4::stage_code_chunk(
                &uploader,
                publisher_addr,
                i,
                create_test_metadata(10 * (i + 1)),
                vector::empty(),
                vector::empty()
            );
            i = i + 1;
        };
        
        // Verify each proposal exists independently
        i = 0;
        while (i < 5) {
            assert!(large_packages_v4::proposal_exists(uploader_addr, publisher_addr, i), i);
            assert!(large_packages_v4::get_proposal_metadata_size(uploader_addr, publisher_addr, i) == 10 * (i + 1), i + 10);
            i = i + 1;
        };
    }

    #[test] 
    fun test_different_uploaders_same_publisher() {
        let (_framework, uploader1, publisher) = setup_test_accounts();
        let uploader2 = account::create_account_for_test(@0xDEAD);
        let publisher_addr = signer::address_of(&publisher);
        let uploader1_addr = signer::address_of(&uploader1);
        let uploader2_addr = signer::address_of(&uploader2);
        
        // Both uploaders create proposals for the same publisher
        large_packages_v4::stage_code_chunk(
            &uploader1,
            publisher_addr,
            1,
            create_test_metadata(100),
            vector::empty(),
            vector::empty()
        );
        
        large_packages_v4::stage_code_chunk(
            &uploader2,
            publisher_addr,
            1, // Same proposal ID but different uploader
            create_test_metadata(200),
            vector::empty(),
            vector::empty()
        );
        
        // Verify both proposals exist independently
        assert!(large_packages_v4::proposal_exists(uploader1_addr, publisher_addr, 1), 1);
        assert!(large_packages_v4::proposal_exists(uploader2_addr, publisher_addr, 1), 2);
        assert!(large_packages_v4::get_proposal_metadata_size(uploader1_addr, publisher_addr, 1) == 100, 3);
        assert!(large_packages_v4::get_proposal_metadata_size(uploader2_addr, publisher_addr, 1) == 200, 4);
    }

    #[test]
    fun test_cleanup_idempotency() {
        let (_framework, uploader, publisher) = setup_test_accounts();
        let proposal_id = 8;
        let uploader_addr = signer::address_of(&uploader);
        let publisher_addr = signer::address_of(&publisher);
        
        // Create a proposal
        large_packages_v4::stage_code_chunk(
            &uploader,
            publisher_addr,
            proposal_id,
            create_test_metadata(50),
            vector::empty(),
            vector::empty()
        );
        
        // Cleanup twice - should not fail
        large_packages_v4::cleanup_proposal(
            &uploader,
            uploader_addr,
            publisher_addr,
            proposal_id
        );
        
        large_packages_v4::cleanup_proposal(
            &uploader,
            uploader_addr,
            publisher_addr,
            proposal_id
        );
        
        assert!(!large_packages_v4::proposal_exists(uploader_addr, publisher_addr, proposal_id), 1);
    }
}