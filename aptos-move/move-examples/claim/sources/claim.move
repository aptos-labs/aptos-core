/// Module to allow efficiently check and claim spots in a limited claim list.
/// Developers can include this module in their application and just need to:
/// 1. Change the package name to their own package name.
/// 2. Add friend declarations for modules that need to depend on the claim_list module.
/// 3. Call is_claimed and claim functions in their own code.
module claim::claim_list {
    use aptos_std::smart_table::{Self, SmartTable};

    // Add friend declarations for modules that need to depend on claim_list.

    struct ClaimList has key {
        // Key can also be replaced with address if the claim_list is tracked by user addresses.
        codes_claimed: SmartTable<u64, bool>,
    }

    fun init_module(claim_list_signer: &signer) {
        move_to<ClaimList>(claim_list_signer, ClaimList {
            codes_claimed: smart_table::new(),
        });
    }

    public fun is_claimed(invite_code: u64): bool acquires ClaimList {
        let codes_claimed = &borrow_global<ClaimList>(@claim).codes_claimed;
        smart_table::contains(codes_claimed, invite_code)
    }

    public(friend) fun claim(invite_code: u64) acquires ClaimList {
        let codes_claimed = &mut borrow_global_mut<ClaimList>(@claim).codes_claimed;
        smart_table::add(codes_claimed, invite_code, true);
    }

    #[test_only]
    friend claim::test_claim;

    #[test_only]
    public fun init_for_test(claim: &signer) {
        init_module(claim);
    }
}
