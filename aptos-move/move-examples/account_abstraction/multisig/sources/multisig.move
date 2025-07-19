module aa::multisig {
    use std::signer;
    use std::string::String;
    use aptos_framework::auth_data::AbstractionAuthData;
    use aptos_framework::transaction_context;
    use aptos_framework::big_ordered_map::{Self, BigOrderedMap};
    use aptos_framework::ordered_map::{Self, OrderedMap};
    use aptos_framework::event;

    const ENOT_AA_MULTISIG_ACCOUNT: u64 = 1;
    const ENOT_OWNER: u64 = 2;
    const EINVALID_THRESHOLD: u64 = 3;
    const EALREADY_VOTED: u64 = 4;
    const ENOT_ENTRY_FUNCTION_PAYLOAD: u64 = 5;
    const ENO_SUCH_PROPOSAL: u64 = 6;
    const ENOT_ENOUGH_VOTES: u64 = 7;

    #[event]
    struct ProposalVoteUpdated has store, drop {
        multisig_account: address,
        proposal: EntryFunctionPayload,
        is_new_proposal: bool,
        owner: address,
        vote: bool,
    }

    #[event]
    struct MultisigAccountReset has store, drop {
        multisig_account: address,
        owners: vector<address>,
        threshold: u64,
    }

    #[event]
    struct ProposalExecuted has store, drop {
        multisig_account: address,
        proposal: EntryFunctionPayload,
        votes: OrderedMap<address, bool>,
        approval_threshold: u64,
    }

    #[event]
    struct ProposalVetoed has store, drop {
        multisig_account: address,
        proposal: EntryFunctionPayload,
        votes: OrderedMap<address, bool>,
        veto_threshold: u64,
    }

    struct EntryFunctionPayload has key, store, copy, drop {
        account_address: address,
        module_name: String,
        function_name: String,
        ty_args_names: vector<String>,
        args: vector<vector<u8>>,
    }

    struct MultisigSettings has key {
        owners: OrderedMap<address, bool>,
        threshold: u64,
        proposals: BigOrderedMap<EntryFunctionPayload, OrderedMap<address, bool>>
    }

    public entry fun reset(aa: &signer, owners: vector<address>, threshold: u64) acquires MultisigSettings {
        let addr = signer::address_of(aa);
        if (exists<MultisigSettings>(addr)) {
            let MultisigSettings { owners: _, threshold: _, proposals } = move_from<MultisigSettings>(addr);
            proposals.destroy(|_| {});
        };
        assert!(threshold > 0 && owners.length() >= threshold, EINVALID_THRESHOLD);
        let dummy_vector = owners.map(|_| true);
        move_to(aa, MultisigSettings {
            owners: ordered_map::new_from(owners, dummy_vector),
            threshold: threshold,
            proposals: big_ordered_map::new_with_type_size_hints(100, 200, 100, 200)
        });
    }

    inline fun ensure_multisig_account(aa: address): &mut MultisigSettings {
        assert!(exists<MultisigSettings>(aa), ENOT_AA_MULTISIG_ACCOUNT);
        &mut MultisigSettings[aa]
    }

    public entry fun vote(owner: &signer, aa: address, account_address: address, module_name: String, function_name: String, ty_args_names: vector<String>, args: vector<vector<u8>>, vote: bool) acquires MultisigSettings {
        let addr = signer::address_of(owner);
        let multisig_settings = ensure_multisig_account(aa);
        assert!(multisig_settings.owners.contains(&addr), ENOT_OWNER);
        let payload = EntryFunctionPayload {
            account_address,
            module_name,
            function_name,
            ty_args_names,
            args,
        };
        if (multisig_settings.proposals.contains(&payload)) {
            let votes = multisig_settings.proposals.remove(&payload);
            if (votes.contains(&addr) && votes.borrow(&addr) == &vote) {
                abort EALREADY_VOTED;
            } else {
                votes.upsert(addr, vote);
                multisig_settings.proposals.add(payload, votes);
                event::emit(ProposalVoteUpdated {
                    multisig_account: aa,
                    proposal: payload,
                    is_new_proposal: false,
                    owner: addr,
                    vote,
                });
            };
        } else {
            let map = ordered_map::new();
            map.add(addr, vote);
            multisig_settings.proposals.add(payload, map);
            event::emit(ProposalVoteUpdated {
                multisig_account: aa,
                proposal: payload,
                is_new_proposal: true,
                owner: addr,
                vote,
            });
        };
        let veto_threshold = multisig_settings.owners.length() - multisig_settings.threshold;
        let votes = *multisig_settings.proposals.borrow(&payload);
        if (votes.values().fold(0, |sum, v| sum + if (!v) {1} else {0}) > veto_threshold) {
            big_ordered_map::remove(&mut multisig_settings.proposals, &payload);
            event::emit(ProposalVetoed {
                multisig_account: aa,
                proposal: payload,
                votes,
                veto_threshold,
            });
        };
    }

    /// Authorization function for account abstraction.
    public fun authenticate(
        account: signer,
        _signing_data: AbstractionAuthData,
    ): signer acquires MultisigSettings {
        let aa_address = signer::address_of(&account);
        let multisig_settings = ensure_multisig_account(aa_address);
        let txn_payload = transaction_context::entry_function_payload();
        assert!(txn_payload.is_some(), ENOT_ENTRY_FUNCTION_PAYLOAD);
        let txn_payload = txn_payload.destroy_some();
        let payload = EntryFunctionPayload {
            account_address: transaction_context::account_address(&txn_payload),
            module_name: transaction_context::module_name(&txn_payload),
            function_name: transaction_context::function_name(&txn_payload),
            ty_args_names: transaction_context::type_arg_names(&txn_payload),
            args: transaction_context::args(&txn_payload),
        };
        assert!(multisig_settings.proposals.contains(&payload), ENO_SUCH_PROPOSAL);
        let votes = multisig_settings.proposals.borrow(&payload);
        assert!(votes.values().fold(0, |sum, v| sum + if (v) {1} else {0}) >= multisig_settings.threshold, ENOT_ENOUGH_VOTES);
        // clean up the proposal before executing the proposal
        multisig_settings.proposals.remove(&payload);
        event::emit(ProposalExecuted {
            multisig_account: aa_address,
            proposal: payload,
            votes,
            approval_threshold: multisig_settings.threshold,
        });
        account
    }

    public entry fun cleanup(admin: &signer) acquires MultisigSettings {
        let addr = signer::address_of(admin);
        if (exists<MultisigSettings>(addr)) {
            let MultisigSettings { owners: _, threshold: _, proposals } = move_from<MultisigSettings>(addr);
            proposals.destroy(|_| {});
        };
    }

    // Test accounts
    const OWNER1: address = @0x123;
    const OWNER2: address = @0x456;
    const OWNER3: address = @0x789;
    const NON_OWNER: address = @0xabc;

    #[test_only]
    use aptos_framework::account;
    #[test_only]
    use aptos_std::string;

    #[test_only]
    fun setup_test(): (signer, signer, signer, signer) {
        let owner1 = account::create_account_for_test(OWNER1);
        let owner2 = account::create_account_for_test(OWNER2);
        let owner3 = account::create_account_for_test(OWNER3);
        let non_owner = account::create_account_for_test(NON_OWNER);
        (owner1, owner2, owner3, non_owner)
    }

    #[test]
    fun test_reset_multisig_basic() acquires MultisigSettings {
        let (owner1, _, _, _) = setup_test();

        // Test valid reset with single owner
        let owners = vector[OWNER1];
        reset(&owner1, owners, 1);

        // Verify the multisig settings
        let settings = &mut MultisigSettings[OWNER1];
        assert!(ordered_map::length(&settings.owners) == 1, 0);
        assert!(settings.threshold == 1, 1);
        assert!(big_ordered_map::is_empty(&settings.proposals), 2);
    }

    #[test]
    fun test_reset_multisig_multiple_owners() acquires MultisigSettings {
        let (owner1, _owner2, _owner3, _) = setup_test();

        // Test valid reset with multiple owners
        let owners = vector[OWNER1, OWNER2, OWNER3];
        reset(&owner1, owners, 2);

        // Verify the multisig settings
        let settings = &mut MultisigSettings[OWNER1];
        assert!(ordered_map::length(&settings.owners) == 3, 0);
        assert!(settings.threshold == 2, 1);
        assert!(big_ordered_map::is_empty(&settings.proposals), 2);

        // Verify all owners are present
        assert!(ordered_map::contains(&settings.owners, &OWNER1), 3);
        assert!(ordered_map::contains(&settings.owners, &OWNER2), 4);
        assert!(ordered_map::contains(&settings.owners, &OWNER3), 5);
    }

    #[test]
    #[expected_failure(abort_code = EINVALID_THRESHOLD)]
    fun test_reset_invalid_threshold_zero() acquires MultisigSettings {
        let (owner1, _, _, _) = setup_test();
        let owners = vector[OWNER1];
        reset(&owner1, owners, 0); // Should fail with invalid threshold
    }

    #[test]
    #[expected_failure(abort_code = EINVALID_THRESHOLD)]
    fun test_reset_threshold_greater_than_owners() acquires MultisigSettings {
        let (owner1, _, _, _) = setup_test();
        let owners = vector[OWNER1, OWNER2];
        reset(&owner1, owners, 3); // Threshold > number of owners
    }

    #[test]
    fun test_reset_overwrites_existing() acquires MultisigSettings {
        let (owner1, _, _, _) = setup_test();

        // First reset
        let owners1 = vector[OWNER1];
        reset(&owner1, owners1, 1);

        // Second reset with different configuration
        let owners2 = vector[OWNER1, OWNER2];
        reset(&owner1, owners2, 2);

        // Verify the new configuration
        let settings = &mut MultisigSettings[OWNER1];
        assert!(ordered_map::length(&settings.owners) == 2, 0);
        assert!(settings.threshold == 2, 1);
    }

    #[test]
    fun test_vote_new_proposal() acquires MultisigSettings {
        let (owner1, _, _, _) = setup_test();

        // Setup multisig
        let owners = vector[OWNER1];
        reset(&owner1, owners, 1);

        // Create a test proposal
        let module_name = string::utf8(b"test_module");
        let function_name = string::utf8(b"test_function");
        let ty_args = vector<String>[];
        let args = vector<vector<u8>>[];

        // Vote on proposal
        vote(&owner1, OWNER1, OWNER1, module_name, function_name, ty_args, args, true);

        // Verify proposal exists and has correct vote
        let settings = &mut MultisigSettings[OWNER1];
        let payload = EntryFunctionPayload {
            account_address: OWNER1,
            module_name,
            function_name,
            ty_args_names: ty_args,
            args,
        };
        assert!(big_ordered_map::contains(&settings.proposals, &payload), 0);
        let votes = big_ordered_map::borrow(&settings.proposals, &payload);
        assert!(ordered_map::length(votes) == 1, 1);
        assert!(*ordered_map::borrow(votes, &OWNER1), 2);
    }

    #[test]
    fun test_vote_multiple_owners() acquires MultisigSettings {
        let (owner1, owner2, owner3, _) = setup_test();

        // Setup multisig with 3 owners, threshold 2
        let owners = vector[OWNER1, OWNER2, OWNER3];
        reset(&owner1, owners, 2);

        // Create a test proposal
        let module_name = string::utf8(b"test_module");
        let function_name = string::utf8(b"test_function");
        let ty_args = vector<String>[];
        let args = vector<vector<u8>>[];

        // Owner1 votes yes
        vote(&owner1, OWNER1, OWNER1, module_name, function_name, ty_args, args, true);

        // Owner2 votes yes
        vote(&owner2, OWNER1, OWNER1, module_name, function_name, ty_args, args, true);

        // Owner3 votes no
        vote(&owner3, OWNER1, OWNER1, module_name, function_name, ty_args, args, false);

        // Verify all votes are recorded
        let settings = &mut MultisigSettings[OWNER1];
        let payload = EntryFunctionPayload {
            account_address: OWNER1,
            module_name,
            function_name,
            ty_args_names: ty_args,
            args,
        };
        let votes = big_ordered_map::borrow(&settings.proposals, &payload);
        assert!(ordered_map::length(votes) == 3, 0);
        assert!(*ordered_map::borrow(votes, &OWNER1), 1);
        assert!(*ordered_map::borrow(votes, &OWNER2), 2);
        assert!(!*ordered_map::borrow(votes, &OWNER3), 3);
    }

    #[test]
    #[expected_failure(abort_code = ENOT_OWNER)]
    fun test_vote_non_owner() acquires MultisigSettings {
        let (owner1, _, _, non_owner) = setup_test();

        // Setup multisig
        let owners = vector[OWNER1];
        reset(&owner1, owners, 1);

        // Non-owner tries to vote
        let module_name = string::utf8(b"test_module");
        let function_name = string::utf8(b"test_function");
        let ty_args = vector<String>[];
        let args = vector<vector<u8>>[];

        vote(&non_owner, OWNER1, OWNER1, module_name, function_name, ty_args, args, true);
    }

    #[test]
    #[expected_failure(abort_code = EALREADY_VOTED)]
    fun test_double_vote_same_value() acquires MultisigSettings {
        let (owner1, _, _, _) = setup_test();

        // Setup multisig
        let owners = vector[OWNER1];
        reset(&owner1, owners, 1);

        // Owner1 votes twice with same value
        let module_name = string::utf8(b"test_module");
        let function_name = string::utf8(b"test_function");
        let ty_args = vector<String>[];
        let args = vector<vector<u8>>[];

        vote(&owner1, OWNER1, OWNER1, module_name, function_name, ty_args, args, true);
        vote(&owner1, OWNER1, OWNER1, module_name, function_name, ty_args, args, true);
    }

    #[test]
    fun test_change_vote() acquires MultisigSettings {
        let (owner1, _, _, _) = setup_test();

        // Setup multisig
        let owners = vector[OWNER1, OWNER2];
        reset(&owner1, owners, 1);

        // Owner1 votes yes, then changes to no
        let module_name = string::utf8(b"test_module");
        let function_name = string::utf8(b"test_function");
        let ty_args = vector<String>[];
        let args = vector<vector<u8>>[];

        let payload = EntryFunctionPayload {
            account_address: OWNER1,
            module_name,
            function_name,
            ty_args_names: ty_args,
            args,
        };

        vote(&owner1, OWNER1, OWNER1, module_name, function_name, ty_args, args, true);
        let votes = *MultisigSettings[OWNER1].proposals.borrow(&payload);
        assert!(*ordered_map::borrow(&votes, &OWNER1), 0);

        vote(&owner1, OWNER1, OWNER1, module_name, function_name, ty_args, args, false);

        // Verify vote was changed
        let votes = *MultisigSettings[OWNER1].proposals.borrow(&payload);
        assert!(!*ordered_map::borrow(&votes, &OWNER1), 0);
    }

    #[test]
    fun test_proposal_veto() acquires MultisigSettings {
        let (owner1, owner2, owner3, _) = setup_test();

        // Setup multisig with 3 owners, threshold 2 (veto threshold = 2)
        let owners = vector[OWNER1, OWNER2, OWNER3];
        reset(&owner1, owners, 2);

        // Create a test proposal
        let module_name = string::utf8(b"test_module");
        let function_name = string::utf8(b"test_function");
        let ty_args = vector<String>[];
        let args = vector<vector<u8>>[];

        // Owner1 votes yes
        vote(&owner1, OWNER1, OWNER1, module_name, function_name, ty_args, args, true);
        // Owner2 votes no (this should veto the proposal)
        vote(&owner2, OWNER1, OWNER1, module_name, function_name, ty_args, args, false);
        // Owner3 votes no (this should veto the proposal)
        vote(&owner3, OWNER1, OWNER1, module_name, function_name, ty_args, args, false);

        // Verify proposal was removed due to veto
        let settings = &mut MultisigSettings[OWNER1];
        let payload = EntryFunctionPayload {
            account_address: OWNER1,
            module_name,
            function_name,
            ty_args_names: ty_args,
            args,
        };
        assert!(!big_ordered_map::contains(&settings.proposals, &payload), 0);
    }

    #[test]
    fun test_cleanup_basic() acquires MultisigSettings {
        let (owner1, _, _, _) = setup_test();

        // Setup multisig
        let owners = vector[OWNER1];
        reset(&owner1, owners, 1);

        // Create a proposal
        let module_name = string::utf8(b"test_module");
        let function_name = string::utf8(b"test_function");
        let ty_args = vector<String>[];
        let args = vector<vector<u8>>[];

        vote(&owner1, OWNER1, OWNER1, module_name, function_name, ty_args, args, true);

        // Cleanup
        cleanup(&owner1);

        // Verify cleanup
        assert!(!exists<MultisigSettings>(OWNER1), 0);
    }

    #[test]
    fun test_cleanup_empty_multisig() acquires MultisigSettings {
        let (owner1, _, _, _) = setup_test();

        // Setup multisig without any proposals
        let owners = vector[OWNER1];
        reset(&owner1, owners, 1);

        // Cleanup should work without error
        cleanup(&owner1);

        // Verify cleanup
        assert!(!exists<MultisigSettings>(OWNER1), 0);
    }

    #[test]
    fun test_multiple_proposals() acquires MultisigSettings {
        let (owner1, _, _, _) = setup_test();

        // Setup multisig
        let owners = vector[OWNER1, OWNER2];
        reset(&owner1, owners, 1);

        // Create two different proposals
        let module1 = string::utf8(b"module1");
        let function1 = string::utf8(b"function1");
        let module2 = string::utf8(b"module2");
        let function2 = string::utf8(b"function2");
        let ty_args = vector<String>[];
        let args = vector<vector<u8>>[];

        // Vote on first proposal
        vote(&owner1, OWNER1, OWNER1, module1, function1, ty_args, args, true);

        // Vote on second proposal
        vote(&owner1, OWNER1, OWNER1, module2, function2, ty_args, args, false);

        // Verify both proposals exist
        let settings = &mut MultisigSettings[OWNER1];
        let payload1 = EntryFunctionPayload {
            account_address: OWNER1,
            module_name: module1,
            function_name: function1,
            ty_args_names: ty_args,
            args,
        };
        let payload2 = EntryFunctionPayload {
            account_address: OWNER1,
            module_name: module2,
            function_name: function2,
            ty_args_names: ty_args,
            args,
        };

        assert!(big_ordered_map::contains(&settings.proposals, &payload1), 0);
        assert!(big_ordered_map::contains(&settings.proposals, &payload2), 1);

        // Verify votes are correct
        let votes1 = big_ordered_map::borrow(&settings.proposals, &payload1);
        let votes2 = big_ordered_map::borrow(&settings.proposals, &payload2);
        assert!(*ordered_map::borrow(votes1, &OWNER1), 2);
        assert!(!*ordered_map::borrow(votes2, &OWNER1), 3);
    }
}
