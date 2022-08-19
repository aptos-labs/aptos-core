module aptos_framework::account {
    use std::bcs;
    use std::error;
    use std::hash;
    use std::option::{Self, Option};
    use std::signer;
    use std::vector;
    use aptos_std::event::{Self, EventHandle};
    use aptos_std::type_info::{Self, TypeInfo};
    use aptos_framework::aptos_coin::AptosCoin;
    use aptos_framework::chain_id;
    use aptos_framework::coin;
    use aptos_framework::system_addresses;
    use aptos_framework::timestamp;
    use aptos_framework::transaction_fee;
    use aptos_std::table::{Self, Table};
    use aptos_std::ed25519;

    friend aptos_framework::coins;
    friend aptos_framework::genesis;

    /// Resource representing an account.
    struct Account has key, store {
        authentication_key: vector<u8>,
        sequence_number: u64,
        coin_register_events: EventHandle<CoinRegisterEvent>,
        rotation_capability_offer: CapabilityOffer<RotationCapability>,
        signer_capability_offer: CapabilityOffer<SignerCapability>,
    }

    struct CoinRegisterEvent has drop, store {
        type_info: TypeInfo,
    }

    /// This holds information that will be picked up by the VM to call the
    /// correct chain-specific prologue and epilogue functions
    struct ChainSpecificAccountInfo has key {
        module_addr: address,
        module_name: vector<u8>,
        script_prologue_name: vector<u8>,
        module_prologue_name: vector<u8>,
        writeset_prologue_name: vector<u8>,
        multi_agent_prologue_name: vector<u8>,
        user_epilogue_name: vector<u8>,
        writeset_epilogue_name: vector<u8>,
    }

    struct CapabilityOffer<phantom T> has store { for: Option<address> }
    struct RotationCapability has drop, store { account: address }
    struct SignerCapability has drop, store { account: address }

    struct OriginatingAddress has key {
        address_map: Table<address, address>,
    }

    /// This structs stores the challenge message that should be signed during key rotation. First, this struct is
    /// signed by the account owner's current public key, which proves possession of a capability to rotate the key.
    /// Second, this struct is signed by the new public key that the account owner wants to rotate to, which proves
    /// knowledge of this new public key's associated secret key. These two signatures cannot be replayed in another
    /// context because they include the TXN's unique sequence number.
    struct RotationProofChallenge has copy, drop {
        sequence_number: u64,
        originator: address, // originating address
        current_auth_key: address, // current auth key
        new_public_key: vector<u8>,
    }

    const MAX_U64: u128 = 18446744073709551615;

    /// Account already exists
    const EACCOUNT_ALREADY_EXISTS: u64 = 1;
    /// Account does not exist
    const EACCOUNT_DOES_NOT_EXIST: u64 = 2;
    /// Sequence number exceeds the maximum value for a u64
    const ESEQUENCE_NUMBER_TOO_BIG: u64 = 3;
    /// The provided authentication key has an invalid length
    const EMALFORMED_AUTHENTICATION_KEY: u64 = 4;
    /// Cannot create account because address is reserved
    const ECANNOT_RESERVED_ADDRESS: u64 = 5;
    /// Transaction exceeded its allocated max gas
    const EOUT_OF_GAS: u64 = 6;
    /// Writesets are not allowed
    const EWRITESET_NOT_ALLOWED: u64 = 7;
    /// Specified current public key is not correct
    const EWRONG_CURRENT_PUBLIC_KEY: u64 = 8;
    /// Specified proof of knowledge required to prove ownership of a public key is invalid
    const EINVALID_PROOF_OF_KNOWLEDGE: u64 = 9;
    /// The caller does not have a digital-signature-based capability to call this function
    const ENO_CAPABILITY: u64 = 10;

    /// Prologue errors. These are separated out from the other errors in this
    /// module since they are mapped separately to major VM statuses, and are
    /// important to the semantics of the system.
    const PROLOGUE_EINVALID_ACCOUNT_AUTH_KEY: u64 = 1001;
    const PROLOGUE_ESEQUENCE_NUMBER_TOO_OLD: u64 = 1002;
    const PROLOGUE_ESEQUENCE_NUMBER_TOO_NEW: u64 = 1003;
    const PROLOGUE_EACCOUNT_DOES_NOT_EXIST: u64 = 1004;
    const PROLOGUE_ECANT_PAY_GAS_DEPOSIT: u64 = 1005;
    const PROLOGUE_ETRANSACTION_EXPIRED: u64 = 1006;
    const PROLOGUE_EBAD_CHAIN_ID: u64 = 1007;
    const PROLOGUE_EINVALID_WRITESET_SENDER: u64 = 1008;
    const PROLOGUE_ESEQUENCE_NUMBER_TOO_BIG: u64 = 1009;
    const PROLOGUE_ESECONDARY_KEYS_ADDRESSES_COUNT_MISMATCH: u64 = 1010;

    #[test_only]
    public fun create_address_for_test(bytes: vector<u8>): address {
        create_address(bytes)
    }

    native fun create_address(bytes: vector<u8>): address;
    native fun create_signer(addr: address): signer;

    public(friend) fun initialize(
        account: &signer,
        module_addr: address,
        module_name: vector<u8>,
        script_prologue_name: vector<u8>,
        module_prologue_name: vector<u8>,
        writeset_prologue_name: vector<u8>,
        multi_agent_prologue_name: vector<u8>,
        user_epilogue_name: vector<u8>,
        writeset_epilogue_name: vector<u8>,
    ) {
        system_addresses::assert_aptos_framework(account);

        move_to(account, ChainSpecificAccountInfo {
            module_addr,
            module_name,
            script_prologue_name,
            module_prologue_name,
            writeset_prologue_name,
            multi_agent_prologue_name,
            user_epilogue_name,
            writeset_epilogue_name,
        });
    }

    // This should only be called during genesis.
    public(friend) fun create_address_map(aptos_framework_account: &signer) {
        system_addresses::assert_aptos_framework(aptos_framework_account);

        move_to(aptos_framework_account, OriginatingAddress {
            address_map: table::new(),
        });
    }

    /// Publishes a new `Account` resource under `new_address`. A signer representing `new_address`
    /// is returned. This way, the caller of this function can publish additional resources under
    /// `new_address`.
    public(friend) fun create_account_internal(new_address: address): signer {
        // there cannot be an Account resource under new_addr already.
        assert!(!exists<Account>(new_address), error::already_exists(EACCOUNT_ALREADY_EXISTS));
        assert!(
            new_address != @vm_reserved && new_address != @aptos_framework,
            error::invalid_argument(ECANNOT_RESERVED_ADDRESS)
        );

        create_account_unchecked(new_address)
    }

    fun create_account_unchecked(new_address: address): signer {
        let new_account = create_signer(new_address);
        let authentication_key = bcs::to_bytes(&new_address);
        assert!(
            vector::length(&authentication_key) == 32,
            error::invalid_argument(EMALFORMED_AUTHENTICATION_KEY)
        );
        move_to(
            &new_account,
            Account {
                authentication_key,
                sequence_number: 0,
                coin_register_events: event::new_event_handle<CoinRegisterEvent>(&new_account),
                rotation_capability_offer: CapabilityOffer { for: option::none() },
                signer_capability_offer: CapabilityOffer { for: option::none() },
            }
        );

        new_account
    }

    public fun exists_at(addr: address): bool {
        exists<Account>(addr)
    }

    public fun get_sequence_number(addr: address) : u64 acquires Account {
        borrow_global<Account>(addr).sequence_number
    }

    public fun get_authentication_key(addr: address) : vector<u8> acquires Account {
        *&borrow_global<Account>(addr).authentication_key
    }

    public entry fun rotate_authentication_key(account: &signer, new_auth_key: vector<u8>) acquires Account {
        rotate_authentication_key_internal(account, new_auth_key);
    }

    public fun rotate_authentication_key_internal(
        account: &signer,
        new_auth_key: vector<u8>,
    ) acquires Account {
        let addr = signer::address_of(account);
        assert!(exists_at(addr), error::not_found(EACCOUNT_ALREADY_EXISTS));
        assert!(
            vector::length(&new_auth_key) == 32,
            error::invalid_argument(EMALFORMED_AUTHENTICATION_KEY)
        );
        let account_resource = borrow_global_mut<Account>(addr);
        account_resource.authentication_key = new_auth_key;
    }

    /// Rotates the authentication key and records a mapping on chain from the new authentication key to the originating
    /// address of the account. To authorize the rotation, a signature under the old public key on a `RotationProofChallenge`
    /// is given in `current_sig`. To ensure the account owner knows the secret key corresponding to the new public key
    /// in `new_pubkey`, a proof-of-knowledge is given in `new_sig` (i.e., a signature under the new public key on the
    /// same `RotationProofChallenge` struct).
    public entry fun rotate_authentication_key_ed25519(
        account: &signer,
        curr_sig_bytes: vector<u8>,
        new_sig_bytes: vector<u8>,
        curr_pk_bytes: vector<u8>,
        new_pk_bytes: vector<u8>,
    ) acquires Account, OriginatingAddress {
        // Get the originating address of the account owner
        let addr = signer::address_of(account);
        assert!(exists_at(addr), error::not_found(EACCOUNT_DOES_NOT_EXIST));
        let curr_pubkey = ed25519::new_unvalidated_public_key_from_bytes(curr_pk_bytes);
        let new_pubkey = ed25519::new_unvalidated_public_key_from_bytes(new_pk_bytes);
        let new_sig = ed25519::new_signature_from_bytes(new_sig_bytes);
        let curr_sig = ed25519::new_signature_from_bytes(curr_sig_bytes);

        // Get the current authentication key of the account
        let account_resource = borrow_global_mut<Account>(addr);
        let curr_auth_key = create_address(account_resource.authentication_key);

        // Check current_pk matches current_auth_key:
        //  1. First, append the Ed25519 scheme identifier '0x00' to the PK
        //  2. Second, hash this using SHA3-256
        vector::push_back(&mut curr_pk_bytes, 0);
        let expected_current_auth_key = hash::sha3_256(curr_pk_bytes);
        assert!(create_address(expected_current_auth_key) == curr_auth_key, std::error::unauthenticated(EWRONG_CURRENT_PUBLIC_KEY));

        // Construct a RotationProofChallenge struct
        let challenge = RotationProofChallenge {
            sequence_number: account_resource.sequence_number,
            originator: addr,
            current_auth_key: curr_auth_key,
            new_public_key: new_pk_bytes,
        };

        // Verify a digital-signature-based capability that assures us this key rotation was intended by the account owner
        assert!(ed25519::signature_verify_strict_t(&curr_sig, &curr_pubkey, copy challenge), std::error::permission_denied(ENO_CAPABILITY));
        // Verify a proof-of-knowledge of the new public key we are rotating to
        assert!(ed25519::signature_verify_strict_t(&new_sig, &new_pubkey, challenge), std::error::invalid_argument(EINVALID_PROOF_OF_KNOWLEDGE));

        // Update the originating address map: i.e., set this account's new address to point to the originating address.
        // Begin by removing the entry for the current authentication key, if there is one.
        let address_map = &mut borrow_global_mut<OriginatingAddress>(@aptos_framework).address_map;
        if (table::contains(address_map, curr_auth_key)) {
            table::remove(address_map, curr_auth_key);
        };

        // Derive the authentication key of the new PK
        vector::push_back(&mut new_pk_bytes, 0);
        let new_auth_key = hash::sha3_256(new_pk_bytes);
        let new_address = create_address(new_auth_key);

        // Update the originating address map
        table::add(address_map, new_address, addr);

        // Update the account with the new authentication key
        account_resource.authentication_key = new_auth_key;
    }

    fun prologue_common(
        sender: signer,
        txn_sequence_number: u64,
        txn_authentication_key: vector<u8>,
        txn_gas_price: u64,
        txn_max_gas_units: u64,
        txn_expiration_time: u64,
        chain_id: u8,
    ) acquires Account {
        assert!(
            timestamp::now_seconds() < txn_expiration_time,
            error::invalid_argument(PROLOGUE_ETRANSACTION_EXPIRED),
        );
        let transaction_sender = signer::address_of(&sender);
        assert!(chain_id::get() == chain_id, error::invalid_argument(PROLOGUE_EBAD_CHAIN_ID));
        assert!(exists<Account>(transaction_sender), error::invalid_argument(PROLOGUE_EACCOUNT_DOES_NOT_EXIST));
        let sender_account = borrow_global<Account>(transaction_sender);
        assert!(
            txn_authentication_key == *&sender_account.authentication_key,
            error::invalid_argument(PROLOGUE_EINVALID_ACCOUNT_AUTH_KEY),
        );
        assert!(
            (txn_sequence_number as u128) < MAX_U64,
            error::out_of_range(PROLOGUE_ESEQUENCE_NUMBER_TOO_BIG)
        );

        assert!(
            txn_sequence_number >= sender_account.sequence_number,
            error::invalid_argument(PROLOGUE_ESEQUENCE_NUMBER_TOO_OLD)
        );

        // [PCA12]: Check that the transaction's sequence number matches the
        // current sequence number. Otherwise sequence number is too new by [PCA11].
        assert!(
            txn_sequence_number == sender_account.sequence_number,
            error::invalid_argument(PROLOGUE_ESEQUENCE_NUMBER_TOO_NEW)
        );
        let max_transaction_fee = txn_gas_price * txn_max_gas_units;
        assert!(
            coin::is_account_registered<AptosCoin>(transaction_sender),
            error::invalid_argument(PROLOGUE_ECANT_PAY_GAS_DEPOSIT),
        );
        let balance = coin::balance<AptosCoin>(transaction_sender);
        assert!(balance >= max_transaction_fee, error::invalid_argument(PROLOGUE_ECANT_PAY_GAS_DEPOSIT));
    }

    ///////////////////////////////////////////////////////////////////////////
    // Prologues and epilogues
    ///////////////////////////////////////////////////////////////////////////
    fun module_prologue(
        sender: signer,
        txn_sequence_number: u64,
        txn_public_key: vector<u8>,
        txn_gas_price: u64,
        txn_max_gas_units: u64,
        txn_expiration_time: u64,
        chain_id: u8,
    ) acquires Account {
        prologue_common(sender, txn_sequence_number, txn_public_key, txn_gas_price, txn_max_gas_units, txn_expiration_time, chain_id)
    }

    fun script_prologue(
        sender: signer,
        txn_sequence_number: u64,
        txn_public_key: vector<u8>,
        txn_gas_price: u64,
        txn_max_gas_units: u64,
        txn_expiration_time: u64,
        chain_id: u8,
        _script_hash: vector<u8>,
    ) acquires Account {
        prologue_common(sender, txn_sequence_number, txn_public_key, txn_gas_price, txn_max_gas_units, txn_expiration_time, chain_id)
    }

    fun writeset_prologue(
        _sender: signer,
        _txn_sequence_number: u64,
        _txn_public_key: vector<u8>,
        _txn_expiration_time: u64,
        _chain_id: u8,
    ) {
        assert!(false, error::invalid_argument(PROLOGUE_EINVALID_WRITESET_SENDER));
    }

    fun multi_agent_script_prologue(
        sender: signer,
        txn_sequence_number: u64,
        txn_sender_public_key: vector<u8>,
        secondary_signer_addresses: vector<address>,
        secondary_signer_public_key_hashes: vector<vector<u8>>,
        txn_gas_price: u64,
        txn_max_gas_units: u64,
        txn_expiration_time: u64,
        chain_id: u8,
    ) acquires Account {
        prologue_common(sender, txn_sequence_number, txn_sender_public_key, txn_gas_price, txn_max_gas_units, txn_expiration_time, chain_id);

        let num_secondary_signers = vector::length(&secondary_signer_addresses);

        assert!(
            vector::length(&secondary_signer_public_key_hashes) == num_secondary_signers,
            error::invalid_argument(PROLOGUE_ESECONDARY_KEYS_ADDRESSES_COUNT_MISMATCH),
        );

        let i = 0;
        while (i < num_secondary_signers) {
            let secondary_address = *vector::borrow(&secondary_signer_addresses, i);
            assert!(exists_at(secondary_address), error::invalid_argument(PROLOGUE_EACCOUNT_DOES_NOT_EXIST));

            let signer_account = borrow_global<Account>(secondary_address);
            let signer_public_key_hash = *vector::borrow(&secondary_signer_public_key_hashes, i);
            assert!(
                signer_public_key_hash == *&signer_account.authentication_key,
                error::invalid_argument(PROLOGUE_EINVALID_ACCOUNT_AUTH_KEY),
            );
            i = i + 1;
        }
    }

    fun writeset_epilogue(
        _core_resource: signer,
        _txn_sequence_number: u64,
        _should_trigger_reconfiguration: bool,
    ) {
        assert!(false, error::invalid_argument(EWRITESET_NOT_ALLOWED));
    }

    /// Epilogue function is run after a transaction is successfully executed.
    /// Called by the Adapter
    fun epilogue(
        account: signer,
        _txn_sequence_number: u64,
        txn_gas_price: u64,
        txn_max_gas_units: u64,
        gas_units_remaining: u64
    ) acquires Account {
        assert!(txn_max_gas_units >= gas_units_remaining, error::invalid_argument(EOUT_OF_GAS));
        let gas_used = txn_max_gas_units - gas_units_remaining;

        assert!(
            (txn_gas_price as u128) * (gas_used as u128) <= MAX_U64,
            error::out_of_range(EOUT_OF_GAS)
        );
        let transaction_fee_amount = txn_gas_price * gas_used;
        let addr = signer::address_of(&account);
        // it's important to maintain the error code consistent with vm
        // to do failed transaction cleanup.
        assert!(
            coin::balance<AptosCoin>(addr) >= transaction_fee_amount,
            error::out_of_range(PROLOGUE_ECANT_PAY_GAS_DEPOSIT),
        );
        transaction_fee::burn_fee(addr, transaction_fee_amount);

        let old_sequence_number = get_sequence_number(addr);

        assert!(
            (old_sequence_number as u128) < MAX_U64,
            error::out_of_range(ESEQUENCE_NUMBER_TOO_BIG)
        );

        // Increment sequence number
        let account_resource = borrow_global_mut<Account>(addr);
        account_resource.sequence_number = old_sequence_number + 1;
    }

    ///////////////////////////////////////////////////////////////////////////
    /// Basic account creation methods.
    ///////////////////////////////////////////////////////////////////////////

    public entry fun create_account(auth_key: address) acquires Account {
        let signer = create_account_internal(auth_key);
        coin::register<AptosCoin>(&signer);
        register_coin<AptosCoin>(auth_key);
    }

    /// A resource account is used to manage resources independent of an account managed by a user.
    public fun create_resource_account(
        source: &signer,
        seed: vector<u8>,
    ): (signer, SignerCapability) {
        let bytes = bcs::to_bytes(&signer::address_of(source));
        vector::append(&mut bytes, seed);
        let addr = create_address(hash::sha3_256(bytes));

        let signer = create_account_internal(copy addr);
        let signer_cap = SignerCapability { account: addr };
        (signer, signer_cap)
    }

    /// Create the account for @aptos_framework to help module upgrades on testnet.
    public(friend) fun create_aptos_framework_account(): (signer, SignerCapability) {
        let signer = create_account_unchecked(@aptos_framework);
        let signer_cap = SignerCapability { account: @aptos_framework };
        (signer, signer_cap)
    }

    public entry fun transfer(source: &signer, to: address, amount: u64) acquires Account {
        if(!exists<Account>(to)) {
            create_account(to)
        };
        coin::transfer<AptosCoin>(source, to, amount)
    }

    ///////////////////////////////////////////////////////////////////////////
    /// Coin management methods.
    ///////////////////////////////////////////////////////////////////////////

    public(friend) fun register_coin<CoinType>(account_addr: address) acquires Account {
        let account = borrow_global_mut<Account>(account_addr);
        event::emit_event<CoinRegisterEvent>(
            &mut account.coin_register_events,
            CoinRegisterEvent {
                type_info: type_info::type_of<CoinType>(),
            },
        );
    }

    ///////////////////////////////////////////////////////////////////////////
    /// Capability based functions for efficient use.
    ///////////////////////////////////////////////////////////////////////////

    public fun create_signer_with_capability(capability: &SignerCapability): signer {
        let addr = &capability.account;
        create_signer(*addr)
    }

    #[test(user = @0x1)]
    public entry fun test_create_resource_account(user: signer) {
        let (resource_account, _) = create_resource_account(&user, x"01");
        assert!(signer::address_of(&resource_account) != signer::address_of(&user), 0);
        coin::register<AptosCoin>(&resource_account);
    }

    #[test_only]
    struct DummyResource has key { }

    #[test(user = @0x1)]
    public entry fun test_module_capability(user: signer) acquires DummyResource {
        let (resource_account, signer_cap) = create_resource_account(&user, x"01");
        assert!(signer::address_of(&resource_account) != signer::address_of(&user), 0);

        let resource_account_from_cap = create_signer_with_capability(&signer_cap);
        assert!(&resource_account == &resource_account_from_cap, 1);
        coin::register<AptosCoin>(&resource_account_from_cap);

        move_to(&resource_account_from_cap, DummyResource { });
        borrow_global<DummyResource>(signer::address_of(&resource_account));
    }

    ///////////////////////////////////////////////////////////////////////////
    // Test-only sequence number mocking for extant Account resource
    ///////////////////////////////////////////////////////////////////////////

    #[test_only]
    /// Increment sequence number of account at address `addr`
    public fun increment_sequence_number(
        addr: address,
    ) acquires Account {
        let acct = borrow_global_mut<Account>(addr);
        acct.sequence_number = acct.sequence_number + 1;
    }

    #[test_only]
    /// Update address `addr` to have `s` as its sequence number
    public fun set_sequence_number(
        addr: address,
        s: u64
    ) acquires Account {
        borrow_global_mut<Account>(addr).sequence_number = s;
    }

    #[test_only]
    public fun create_test_signer_cap(account: address): SignerCapability {
        SignerCapability { account }
    }

    #[test]
    /// Verify test-only sequence number mocking
    public entry fun mock_sequence_numbers()
    acquires Account {
        let addr: address = @0x1234; // Define test address
        create_account(addr); // Initialize account resource
        // Assert sequence number intializes to 0
        assert!(borrow_global<Account>(addr).sequence_number == 0, 0);
        increment_sequence_number(addr); // Increment sequence number
        // Assert correct mock value post-increment
        assert!(borrow_global<Account>(addr).sequence_number == 1, 1);
        set_sequence_number(addr, 10); // Set mock sequence number
        // Assert correct mock value post-modification
        assert!(borrow_global<Account>(addr).sequence_number == 10, 2);
    }

    ///////////////////////////////////////////////////////////////////////////
    // Test account helpers
    ///////////////////////////////////////////////////////////////////////////

    #[test(alice = @0xa11ce, core = @0x1)]
    public fun test_transfer(alice: signer, core: signer) acquires Account {
        let bob = create_address(x"0000000000000000000000000000000000000000000000000000000000000b0b");
        let carol = create_address(x"00000000000000000000000000000000000000000000000000000000000ca501");

        let (burn_cap, mint_cap) = aptos_framework::aptos_coin::initialize_for_test(&core);
        create_account(signer::address_of(&alice));
        coin::deposit(signer::address_of(&alice), coin::mint(10000, &mint_cap));
        transfer(&alice, bob, 500);
        assert!(coin::balance<AptosCoin>(bob) == 500, 0);
        transfer(&alice, carol, 500);
        assert!(coin::balance<AptosCoin>(carol) == 500, 1);
        transfer(&alice, carol, 1500);
        assert!(coin::balance<AptosCoin>(carol) == 2000, 2);

        coin::destroy_burn_cap(burn_cap);
        coin::destroy_mint_cap(mint_cap);
        let _bob = bob;
    }

    #[test(alice = @0xa11ce)]
    #[expected_failure(abort_code = 65537)]
    public entry fun test_empty_public_key(alice: signer) acquires Account, OriginatingAddress {
        create_account(signer::address_of(&alice));
        let pk = vector::empty<u8>();
        let sig = x"00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000";
        rotate_authentication_key_ed25519(&alice, sig, sig, pk, pk);
    }

    #[test(alice = @0xa11ce)]
    #[expected_failure(abort_code = 65538)]
    public entry fun test_empty_signature(alice: signer) acquires Account, OriginatingAddress {
        create_account(signer::address_of(&alice));
        let test_signature  = vector::empty<u8>();
        let pk = x"0000000000000000000000000000000000000000000000000000000000000000";
        rotate_authentication_key_ed25519(&alice, test_signature, test_signature, pk, pk);
    }
}
