module aptos_framework::account {
    use std::bcs;
    use std::error;
    use std::hash;
    use std::option::{Self, Option};
    use std::signer;
    use std::vector;
    use aptos_std::type_info::{Self, TypeInfo};
    use aptos_framework::byte_conversions;
    use aptos_framework::event::{Self, EventHandle};
    use aptos_framework::guid;
    use aptos_framework::system_addresses;
    use aptos_std::table::{Self, Table};
    use aptos_std::ed25519;

    friend aptos_framework::aptos_account;
    friend aptos_framework::coin;
    friend aptos_framework::genesis;
    friend aptos_framework::resource_account;
    friend aptos_framework::transaction_validation;

    /// Resource representing an account.
    struct Account has key, store {
        authentication_key: vector<u8>,
        sequence_number: u64,
        guid_creation_num: u64,
        coin_register_events: EventHandle<CoinRegisterEvent>,
        key_rotation_events: EventHandle<KeyRotationEvent>,
        rotation_capability_offer: CapabilityOffer<RotationCapability>,
        signer_capability_offer: CapabilityOffer<SignerCapability>,
    }

    struct KeyRotationEvent has drop, store {
        old_authentication_key: vector<u8>,
        new_authentication_key: vector<u8>,
    }

    struct CoinRegisterEvent has drop, store {
        type_info: TypeInfo,
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

    struct RotationCapabilityOfferProofChallenge has drop {
        sequence_number: u64,
        recipient_address: address,
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
    /// Specified current public key is not correct
    const EWRONG_CURRENT_PUBLIC_KEY: u64 = 7;
    /// Specified proof of knowledge required to prove ownership of a public key is invalid
    const EINVALID_PROOF_OF_KNOWLEDGE: u64 = 8;
    /// The caller does not have a digital-signature-based capability to call this function
    const ENO_CAPABILITY: u64 = 9;
    /// The caller does not have a valid rotation capability offer from the other account
    const EINVALID_ACCEPT_ROTATION_CAPABILITY: u64 = 10;
    ///
    const ENO_VALID_FRAMEWORK_RESERVED_ADDRESS: u64 = 11;

    native fun create_signer(addr: address): signer;

    /// Only called during genesis to initialize system resources for this module.
    public(friend) fun initialize(aptos_framework: &signer) {
        system_addresses::assert_aptos_framework(aptos_framework);
        move_to(aptos_framework, OriginatingAddress {
            address_map: table::new(),
        });
    }

    /// Publishes a new `Account` resource under `new_address`. A signer representing `new_address`
    /// is returned. This way, the caller of this function can publish additional resources under
    /// `new_address`.
    public(friend) fun create_account(new_address: address): signer {
        // there cannot be an Account resource under new_addr already.
        assert!(!exists<Account>(new_address), error::already_exists(EACCOUNT_ALREADY_EXISTS));
        assert!(
            new_address != @vm_reserved && new_address != @aptos_framework,
            error::invalid_argument(ECANNOT_RESERVED_ADDRESS)
        );

        create_account_unchecked(new_address)
    }

    #[test_only]
    public fun create_account_for_test(new_address: address): signer {
        create_account_unchecked(new_address)
    }

    fun create_account_unchecked(new_address: address): signer {
        let new_account = create_signer(new_address);
        let authentication_key = byte_conversions::from_address(&new_address);
        assert!(
            vector::length(&authentication_key) == 32,
            error::invalid_argument(EMALFORMED_AUTHENTICATION_KEY)
        );

        let guid_creation_num = 0;

        let guid_for_coin = guid::create(new_address, &mut guid_creation_num);
        let coin_register_events = event::new_event_handle<CoinRegisterEvent>(guid_for_coin);

        let guid_for_rotation = guid::create(new_address, &mut guid_creation_num);
        let key_rotation_events = event::new_event_handle<KeyRotationEvent>(guid_for_rotation);

        move_to(
            &new_account,
            Account {
                authentication_key,
                sequence_number: 0,
                guid_creation_num,
                coin_register_events,
                key_rotation_events,
                rotation_capability_offer: CapabilityOffer { for: option::none() },
                signer_capability_offer: CapabilityOffer { for: option::none() },
            }
        );

        new_account
    }

    public fun exists_at(addr: address): bool {
        exists<Account>(addr)
    }

    public fun get_guid_next_creation_num(addr: address): u64 acquires Account {
        borrow_global<Account>(addr).guid_creation_num
    }

    public fun get_sequence_number(addr: address) : u64 acquires Account {
        borrow_global<Account>(addr).sequence_number
    }

    public(friend) fun increment_sequence_number(addr: address) acquires Account {
        let account_resource = borrow_global_mut<Account>(addr);
        let old_sequence_number = account_resource.sequence_number;

        assert!(
            (old_sequence_number as u128) < MAX_U64,
            error::out_of_range(ESEQUENCE_NUMBER_TOO_BIG)
        );

        account_resource.sequence_number = old_sequence_number + 1;
    }

    public fun get_authentication_key(addr: address) : vector<u8> acquires Account {
        *&borrow_global<Account>(addr).authentication_key
    }

    public(friend) fun rotate_authentication_key_internal(account: &signer, new_auth_key: vector<u8>) acquires Account {
        let addr = signer::address_of(account);
        assert!(exists_at(addr), error::not_found(EACCOUNT_ALREADY_EXISTS));
        assert!(
            vector::length(&new_auth_key) == 32,
            error::invalid_argument(EMALFORMED_AUTHENTICATION_KEY)
        );
        let account_resource = borrow_global_mut<Account>(addr);
        account_resource.authentication_key = new_auth_key;
    }

    // Check account_public_key_bytes matches current_auth_key:
    //  1. First, append the Ed25519 scheme identifier '0x00' to `account_public_key_bytes`
    //  2. Second, hash this using SHA3-256
    fun verify_authentication_key_matches_ed25519_public_key(account_auth_key: vector<u8>, account_public_key_bytes: vector<u8>) : bool {
        vector::push_back(&mut account_public_key_bytes, 0);
        let expected_account_auth_key = hash::sha3_256(account_public_key_bytes);
        expected_account_auth_key == account_auth_key
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

        // Get the current authentication key of the account and verify that it matches with `curr_pk_bytes`
        let account_resource = borrow_global_mut<Account>(addr);
        assert!(verify_authentication_key_matches_ed25519_public_key(account_resource.authentication_key, curr_pk_bytes), std::error::unauthenticated(EWRONG_CURRENT_PUBLIC_KEY));

        let curr_auth_key = byte_conversions::to_address(account_resource.authentication_key);
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
        let new_address = byte_conversions::to_address(new_auth_key);

        // Update the originating address map
        table::add(address_map, new_address, addr);

        // Update the account with the new authentication key
        account_resource.authentication_key = new_auth_key;
    }

    /// Offer rotation capability of this account to another address
    /// To authorize the rotation capability offer, a signature under the current public key on a `RotationCapabilityOfferProofChallenge`
    /// is given in `rotation_capability_sig_bytes`. The current public key is passed into `account_public_key_bytes` to verify proof-of-knowledge.
    /// The recipient address refers to the address that the account owner wants to give the rotation capability to.
    public entry fun offer_rotation_capability_ed25519(
        account: &signer,
        rotation_capability_sig_bytes: vector<u8>,
        account_public_key_bytes: vector<u8>,
        recipient_address: address,
    ) acquires Account {
        let addr = signer::address_of(account);
        assert!(exists_at(addr) && exists_at(recipient_address), error::not_found(EACCOUNT_DOES_NOT_EXIST));

        let pubkey = ed25519::new_unvalidated_public_key_from_bytes(account_public_key_bytes);
        let rotation_capability_sig = ed25519::new_signature_from_bytes(rotation_capability_sig_bytes);

        // Get the current authentication key of the account and verify that it matches with `account_public_key_bytes`
        let account_resource = borrow_global_mut<Account>(addr);
        assert!(verify_authentication_key_matches_ed25519_public_key(account_resource.authentication_key, account_public_key_bytes), EWRONG_CURRENT_PUBLIC_KEY);

        //  Construct a RotationCapabilityOfferProofChallenge struct
        std::debug::print(account_resource);
        let rotation_capability_offer_proof_challenge = RotationCapabilityOfferProofChallenge {
            sequence_number: account_resource.sequence_number,
            recipient_address,
        };

        std::debug::print(&account_resource.sequence_number);
        // Verify a digital-signature-based capability that assures us this rotation capability offer was intended by the account owner
        assert!(ed25519::signature_verify_strict_t(&rotation_capability_sig, &pubkey, rotation_capability_offer_proof_challenge), EINVALID_PROOF_OF_KNOWLEDGE);

        // Add the recipient's address in account owner's rotation capability offer once we verify that this action is intended by the account owner
        option::fill(&mut account_resource.rotation_capability_offer.for, recipient_address);
    }

    // Accept rotation capability from `offerer_address` if there's an existing rotation capability offer to the account owner in the offerer's account
    public fun accept_rotation_capability_ed25519(account: &signer, offerer_address: address) : RotationCapability acquires Account {
        assert!(exists_at(offerer_address), error::not_found(EACCOUNT_DOES_NOT_EXIST));

        // Check if there's an existing rotation capability offer from the offerer
        let account_resource = borrow_global_mut<Account>(offerer_address);
        let addr = signer::address_of(account);
        assert!(option::contains(&account_resource.rotation_capability_offer.for, &addr), EINVALID_ACCEPT_ROTATION_CAPABILITY);

        // If there's an existing rotation capability offer for this account in the offerer's account,
        // we create a RotationCapability of offerer and return the RotationCapability
        let rotation_capability = RotationCapability {
            account: offerer_address,
        };
        option::extract(&mut account_resource.rotation_capability_offer.for);

        rotation_capability
    }

    ///////////////////////////////////////////////////////////////////////////
    /// Basic account creation methods.
    ///////////////////////////////////////////////////////////////////////////

    /// A resource account is used to manage resources independent of an account managed by a user.
    public fun create_resource_account(source: &signer, seed: vector<u8>): (signer, SignerCapability) {
        let bytes = bcs::to_bytes(&signer::address_of(source));
        vector::append(&mut bytes, seed);
        let addr = byte_conversions::to_address(hash::sha3_256(bytes));

        let signer = create_account_unchecked(copy addr);
        let signer_cap = SignerCapability { account: addr };
        (signer, signer_cap)
    }

    /// create the account for system reserved addresses
    public(friend) fun create_framework_reserved_account(addr: address): (signer, SignerCapability) {
        assert!(
            addr == @0x1 ||
            addr == @0x2 ||
            addr == @0x3 ||
            addr == @0x4 ||
            addr == @0x5 ||
            addr == @0x6 ||
            addr == @0x7 ||
            addr == @0x8 ||
            addr == @0x9 ||
            addr == @0x10,
            error::permission_denied(ENO_VALID_FRAMEWORK_RESERVED_ADDRESS),
        );
        let signer = create_account_unchecked(addr);
        let signer_cap = SignerCapability { account: addr };
        (signer, signer_cap)
    }

    ///////////////////////////////////////////////////////////////////////////
    /// GUID management methods.
    ///////////////////////////////////////////////////////////////////////////

    public fun create_guid(account_signer: &signer): guid::GUID acquires Account {
        let addr = signer::address_of(account_signer);
        let account = borrow_global_mut<Account>(addr);
        guid::create(addr, &mut account.guid_creation_num)
    }

    ///////////////////////////////////////////////////////////////////////////
    /// GUID management methods.
    ///////////////////////////////////////////////////////////////////////////

    public fun new_event_handle<T: drop + store>(account: &signer): EventHandle<T> acquires Account {
        event::new_event_handle(create_guid(account))
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
    }

    #[test_only]
    struct DummyResource has key { }

    #[test(user = @0x1)]
    public entry fun test_module_capability(user: signer) acquires DummyResource {
        let (resource_account, signer_cap) = create_resource_account(&user, x"01");
        assert!(signer::address_of(&resource_account) != signer::address_of(&user), 0);

        let resource_account_from_cap = create_signer_with_capability(&signer_cap);
        assert!(&resource_account == &resource_account_from_cap, 1);

        move_to(&resource_account_from_cap, DummyResource { });
        borrow_global<DummyResource>(signer::address_of(&resource_account));
    }

    ///////////////////////////////////////////////////////////////////////////
    // Test-only sequence number mocking for extant Account resource
    ///////////////////////////////////////////////////////////////////////////

    #[test_only]
    /// Increment sequence number of account at address `addr`
    public fun increment_sequence_number_for_test(
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
        increment_sequence_number_for_test(addr); // Increment sequence number
        // Assert correct mock value post-increment
        assert!(borrow_global<Account>(addr).sequence_number == 1, 1);
        set_sequence_number(addr, 10); // Set mock sequence number
        // Assert correct mock value post-modification
        assert!(borrow_global<Account>(addr).sequence_number == 10, 2);
    }

    ///////////////////////////////////////////////////////////////////////////
    // Test account helpers
    ///////////////////////////////////////////////////////////////////////////

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

    #[test(bob = @0x345)]
    #[expected_failure(abort_code = 8)]
    public entry fun test_invalid_offer_rotation_capability(bob: signer) acquires Account {
        let pk_with_scheme = x"f66bf0ce5ceb582b93d6780820c2025b9967aedaa259bdbb9f3d0297eced0e18";
        vector::push_back(&mut pk_with_scheme, 0);
        let alice_address = byte_conversions::to_address(hash::sha3_256(pk_with_scheme));
        let alice = create_account_unchecked(alice_address);
        create_account(signer::address_of(&bob));

        let pk = x"f66bf0ce5ceb582b93d6780820c2025b9967aedaa259bdbb9f3d0297eced0e18";
        let invalid_signature = x"78f7d09ef7a9d8d7450d600b10231e6512610f919a63bd71bea1c907f7e101ed333bff360eeda97a8637a53fd622d597c03a0d6fd1315c6fa23719983ff7de0c";
        offer_rotation_capability_ed25519(&alice, invalid_signature, pk, signer::address_of(&bob));
    }

    #[test(bob = @0x345)]
    public entry fun test_valid_accept_rotation_capability(bob: signer) acquires Account {
        let pk = x"f66bf0ce5ceb582b93d6780820c2025b9967aedaa259bdbb9f3d0297eced0e18";
        let pk_with_scheme = copy pk;
        vector::push_back(&mut pk_with_scheme, 0);
        let alice_address = byte_conversions::to_address(hash::sha3_256(pk_with_scheme));
        let alice = create_account_unchecked(alice_address);
        create_account(signer::address_of(&bob));

        let valid_signature = x"68f7d09ef7a9d8d7450d600b10231e6512610f919a63bd71bea1c907f7e101ed333bff360eeda97a8637a53fd622d597c03a0d6fd1315c6fa23719983ff7de0c";
        offer_rotation_capability_ed25519(&alice, valid_signature, pk, signer::address_of(&bob));

        let alice_account_resource = borrow_global_mut<Account>(signer::address_of(&alice));
        assert!(option::contains(&alice_account_resource.rotation_capability_offer.for, &signer::address_of(&bob)), 0);

        let rotation_cap = accept_rotation_capability_ed25519(&bob, signer::address_of(&alice));
        assert!(rotation_cap.account == signer::address_of(&alice), 0);
    }

    #[test(bob = @0x345, charlie=@0x567)]
    #[expected_failure(abort_code = 10)]
    public entry fun test_invalid_accept_rotation_capability(bob: signer, charlie: signer) acquires Account {
        let pk = x"f66bf0ce5ceb582b93d6780820c2025b9967aedaa259bdbb9f3d0297eced0e18";
        let pk_with_scheme = copy pk;
        vector::push_back(&mut pk_with_scheme, 0);
        let alice_address = byte_conversions::to_address(hash::sha3_256(pk_with_scheme));
        let alice = create_account_unchecked(alice_address);
        create_account(signer::address_of(&bob));
        create_account(signer::address_of(&charlie));

        let valid_signature = x"68f7d09ef7a9d8d7450d600b10231e6512610f919a63bd71bea1c907f7e101ed333bff360eeda97a8637a53fd622d597c03a0d6fd1315c6fa23719983ff7de0c";
        offer_rotation_capability_ed25519(&alice, valid_signature, pk, signer::address_of(&bob));

        let alice_account_resource = borrow_global_mut<Account>(signer::address_of(&alice));
        assert!(option::contains(&alice_account_resource.rotation_capability_offer.for, &signer::address_of(&bob)), 0);

        accept_rotation_capability_ed25519(&bob, signer::address_of(&charlie));
    }
}
