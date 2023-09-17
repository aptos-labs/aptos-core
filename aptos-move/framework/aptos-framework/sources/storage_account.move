/// A storage account is a lightweight way to allocate global storage:
/// - Cheaper than using a table since no handle generation/hashing.
/// - Cheaper than a resource account, which allocates an Account too.
/// - Cheaper than creating an object & immediately deleting ObjectCore.
module aptos_framework::storage_account {
    use aptos_framework::create_signer::create_signer;
    use aptos_framework::transaction_context;

    struct SignerCapability has copy, drop, store { account: address }

    public fun create_storage_account(): (address, signer) {
        let storage_addr = transaction_context::generate_auid_address();
        (storage_addr, create_signer(storage_addr))
    }

    public fun create_storage_account_and_capability(): (
        address,
        signer,
        SignerCapability,
    ) {
        let addr = transaction_context::generate_auid_address();
        (addr, create_signer(addr), SignerCapability { account: addr })
    }

    public fun get_signer_capability_address(cap: &SignerCapability): address {
        cap.account
    }

    public fun create_signer_with_capability(cap: &SignerCapability): signer {
        create_signer(cap.account)
    }

    #[test_only]
    use aptos_framework::type_info;
    #[test_only]
    use std::features;
    #[test_only]
    use std::option::{Self, Option};
    #[test_only]
    use std::signer;

    #[test_only]
    struct TreeRoot has key {
        node: Option<address>,
    }

    #[test_only]
    struct LeafNode has key, drop {
        data: vector<u8>,
    }

    #[test_only]
    /// From gas schedule `transaction.rs`
    /// Ideally this value would be queryable during runtime.
    const FREE_WRITE_BYTES_QUOTA: u64 = 1024;

    #[test_only]
    const DATA: vector<u8> = b"I can store so much data in a storage account because it has no overhead: no object::ObjectCore, no account::Account, just the data I need. When I need to access my state I don't need to do any expensive hashing against a table handle. Rather, I simply borrow out of global storage. Then when I'm done, I simply deallocate from memory using move_from(). Neat! Wow, I sure do have a lot of extra space I can fit into here, so how about some more ASCII characters? Here you go! 000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000";

    #[test(features = @std, tree_creator = @0xace)]
    fun test_end_to_end(
        features: &signer,
        tree_creator: &signer,
    ) acquires TreeRoot, LeafNode {
        // Enable AUID generation for testing.
        let feature = features::get_auids();
        features::change_feature_flags(features, vector[feature], vector[]);
        // Create an empty tree.
        let tree_creator_address = signer::address_of(tree_creator);
        move_to(tree_creator, TreeRoot { node: option::none() });
        // Allocate a new leaf node under a storage account.
        let (storage_addr, storage_account) = create_storage_account();
        move_to(&storage_account, LeafNode { data: DATA });
        let root_ref_mut = borrow_global_mut<TreeRoot>(tree_creator_address);
        root_ref_mut.node = option::some(storage_addr);
        // Check the size of the allocated leaf node.
        let node_ref = borrow_global<LeafNode>(storage_addr);
        let storage_size = type_info::size_of_val(node_ref);
        assert!(storage_size == FREE_WRITE_BYTES_QUOTA, 0);
        // Perform an arbitrary node operation.
        let leaf_ref_mut = borrow_global_mut<LeafNode>(storage_addr);
        leaf_ref_mut.data = b"I sure do like storage accounts!";
        // Delete the node.
        root_ref_mut.node = option::none();
        move_from<LeafNode>(storage_addr);
        // Verify capability functionality.
        let (addr, account, cap) = create_storage_account_and_capability();
        assert!(addr == get_signer_capability_address(&cap), 0);
        assert!(account == create_signer_with_capability(&cap), 0);
    }

}