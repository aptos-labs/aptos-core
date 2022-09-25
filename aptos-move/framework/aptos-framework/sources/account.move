module aptos_framework::account {
    use std::bcs;
    use std::error;
    use std::hash;
    use std::option::{Self, Option};
    use std::signer;
    use std::vector;
    use aptos_std::type_info::{Self, TypeInfo};
    use aptos_framework::event::{Self, EventHandle};
    use aptos_framework::guid;
    use aptos_framework::system_addresses;
    use aptos_std::table::{Self, Table};
    use aptos_std::ed25519;
    use aptos_std::from_bcs;
    use aptos_std::multi_ed25519;

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
        originator: address,
        // originating address
        current_auth_key: address,
        // current auth key
        new_public_key: vector<u8>,
    }

    struct RotationCapabilityOfferProofChallenge has drop {
        sequence_number: u64,
        recipient_address: address,
    }

    struct SignerCapabilityOfferProofChallenge has drop {
        sequence_number: u64,
        recipient_address: address,
    }

    const MAX_U64: u128 = 18446744073709551615;

    const ED25519_SCHEME: u8 = 0;
    const MULTI_ED25519_SCHEME: u8 = 1;

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
    /// Address to create is not a valid reserved address for Aptos framework
    const ENO_VALID_FRAMEWORK_RESERVED_ADDRESS: u64 = 11;
    /// Specified scheme required to proceed with the smart contract operation - can only be ED25519_SCHEME(0) OR MULTI_ED25519_SCHEME(1)
    const EINVALID_SCHEME: u64 = 12;
    /// Abort the transaction if the expected originating address is different from the originating addres on-chain
    const EINVALID_ORIGINATING_ADDRESS: u64 = 13;
    /// The signer capability doesn't exist at the given address
    const ENO_SUCH_SIGNER_CAPABILITY: u64 = 14;

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
        let authentication_key = bcs::to_bytes(&new_address);
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

    public fun get_sequence_number(addr: address): u64 acquires Account {
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

    public fun get_authentication_key(addr: address): vector<u8> acquires Account {
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

    fun verify_key_rotation_signature_and_get_auth_key(scheme: u8, public_key_bytes: vector<u8>, signature: vector<u8>, challenge: &RotationProofChallenge): vector<u8> {
        if (scheme == ED25519_SCHEME) {
            let pk = ed25519::new_unvalidated_public_key_from_bytes(public_key_bytes);
            let sig = ed25519::new_signature_from_bytes(signature);
            assert!(ed25519::signature_verify_strict_t(&sig, &pk, *challenge), std::error::invalid_argument(EINVALID_PROOF_OF_KNOWLEDGE));
            ed25519::unvalidated_public_key_to_authentication_key(&pk)
        } else if (scheme == MULTI_ED25519_SCHEME) {
            let pk = multi_ed25519::new_unvalidated_public_key_from_bytes(public_key_bytes);
            let sig = multi_ed25519::new_signature_from_bytes(signature);
            assert!(multi_ed25519::signature_verify_strict_t(&sig, &pk, *challenge), std::error::invalid_argument(EINVALID_PROOF_OF_KNOWLEDGE));
            multi_ed25519::unvalidated_public_key_to_authentication_key(&pk)
        } else {
            abort error::invalid_argument(EINVALID_SCHEME)
        }
    }

    /// Generic authentication key rotation function that allows the user to rotate their authentication key from any scheme to any scheme.
    /// To authorize the rotation, a signature by the current private key on a valid RotationProofChallenge (`cap_rotate_key`)
    /// demonstrates that the user intends to and has the capability to rotate the authentication key. A signature by the new
    /// private key on a valid RotationProofChallenge (`cap_update_table`) verifies that the user has the capability to update the
    /// value at key `auth_key` on the `OriginatingAddress` table. `from_scheme` refers to the scheme of the `from_public_key` and
    /// `to_scheme` refers to the scheme of the `to_public_key`. A scheme of 0 refers to an Ed25519 key and a scheme of 1 refers to
    /// Multi-Ed25519 keys.
    public entry fun rotate_authentication_key(
        account: &signer,
        from_scheme: u8,
        from_public_key_bytes: vector<u8>,
        to_scheme: u8,
        to_public_key_bytes: vector<u8>,
        cap_rotate_key: vector<u8>,
        cap_update_table: vector<u8>,
    ) acquires Account, OriginatingAddress {
        let addr = signer::address_of(account);
        assert!(exists_at(addr), error::not_found(EACCOUNT_DOES_NOT_EXIST));

        let account_resource = borrow_global_mut<Account>(addr);

        // verify the public key matches the current authentication key
        if (from_scheme == ED25519_SCHEME) {
            let from_pk = ed25519::new_unvalidated_public_key_from_bytes(from_public_key_bytes);
            let from_auth_key = ed25519::unvalidated_public_key_to_authentication_key(&from_pk);
            assert!(account_resource.authentication_key == from_auth_key, error::unauthenticated(EWRONG_CURRENT_PUBLIC_KEY));
        } else if (from_scheme == MULTI_ED25519_SCHEME) {
            let from_pk = multi_ed25519::new_unvalidated_public_key_from_bytes(from_public_key_bytes);
            let from_auth_key = multi_ed25519::unvalidated_public_key_to_authentication_key(&from_pk);
            assert!(account_resource.authentication_key == from_auth_key, error::unauthenticated(EWRONG_CURRENT_PUBLIC_KEY));
        } else {
            abort error::invalid_argument(EINVALID_SCHEME)
        };

        let curr_auth_key = from_bcs::to_address(account_resource.authentication_key);
        // construct a RotationProofChallenge to prove that the user intends to do a key rotation
        let challenge = RotationProofChallenge {
            sequence_number: account_resource.sequence_number,
            originator: addr,
            current_auth_key: curr_auth_key,
            new_public_key: to_public_key_bytes,
        };

        // verify that the challenge signed by the current private key and the previous private key are both valid
        let curr_auth_key = verify_key_rotation_signature_and_get_auth_key(from_scheme, from_public_key_bytes, cap_rotate_key, &challenge);
        let new_auth_key = verify_key_rotation_signature_and_get_auth_key(to_scheme, to_public_key_bytes, cap_update_table, &challenge);

        // update the address_map table, so that we can reference to the originating address using the current address
        let address_map = &mut borrow_global_mut<OriginatingAddress>(@aptos_framework).address_map;
        let curr_address = from_bcs::to_address(curr_auth_key);
        let new_address = from_bcs::to_address(new_auth_key);

        if (table::contains(address_map, curr_address)) {
            // assert that we're calling from the same account of the originating address
            // for example, if we have already rotated from keypair_a to keypair_b, and are trying to rotate from
            // keypair_b to keypair_c, we expect the call to come from the signer of address_a
            assert!(addr == table::remove(address_map, curr_address), error::not_found(EINVALID_ORIGINATING_ADDRESS));
        };
        table::add(address_map, new_address, addr);

        // update the authentication key of the current account
        let account_resource = borrow_global_mut<Account>(addr);

        event::emit_event<KeyRotationEvent>(
            &mut account_resource.key_rotation_events,
            KeyRotationEvent {
                old_authentication_key: account_resource.authentication_key,
                new_authentication_key: new_auth_key,
            }
        );

        account_resource.authentication_key = new_auth_key;
    }

    /// Offers signer capability on behalf of `account` to the account at address `recipient_address`.
    /// An account can delegate its signer capability to only one other address at one time.
    /// `signer_capability_key_bytes` is the `SignerCapabilityOfferProofChallenge` signed by the account owner's key
    /// `account_scheme` is the scheme of the account (ed25519 or multi_ed25519)
    /// `account_public_key_bytes` is the public key of the account owner
    /// `recipient_address` is the address of the recipient of the signer capability - note that if there's an existing
    /// `recipient_address` in the account owner's `SignerCapabilityOffer`, this will replace the
    /// previous `recipient_address` upon successful verification (the previous recipient will no longer have access
    /// to the account owner's signer capability)
    public entry fun offer_signer_capability(
        account: &signer,
        signer_capability_sig_bytes: vector<u8>,
        account_scheme: u8,
        account_public_key_bytes: vector<u8>,
        recipient_address: address
    ) acquires Account {
        let addr = signer::address_of(account);
        assert!(exists_at(addr) && exists_at(recipient_address), error::not_found(EACCOUNT_DOES_NOT_EXIST));

        let account_resource = borrow_global_mut<Account>(addr);
        // proof that this account intends to delegate its signer capability to another account
        let proof_challenge = SignerCapabilityOfferProofChallenge {
            sequence_number: account_resource.sequence_number,
            recipient_address,
        };

        // verify that the `SignerCapabilityOfferProofChallenge` is correct and signed by the account owner's private key
        if (account_scheme == ED25519_SCHEME) {
            let pubkey = ed25519::new_unvalidated_public_key_from_bytes(account_public_key_bytes);
            let expected_auth_key = ed25519::unvalidated_public_key_to_authentication_key(&pubkey);
            assert!(account_resource.authentication_key == expected_auth_key, error::invalid_argument(EWRONG_CURRENT_PUBLIC_KEY));

            let signer_capability_sig = ed25519::new_signature_from_bytes(signer_capability_sig_bytes);
            assert!(ed25519::signature_verify_strict_t(&signer_capability_sig, &pubkey, proof_challenge), error::invalid_argument(EINVALID_PROOF_OF_KNOWLEDGE));
        } else if (account_scheme == MULTI_ED25519_SCHEME) {
            let pubkey = multi_ed25519::new_unvalidated_public_key_from_bytes(account_public_key_bytes);
            let expected_auth_key = multi_ed25519::unvalidated_public_key_to_authentication_key(&pubkey);
            assert!(account_resource.authentication_key == expected_auth_key, error::invalid_argument(EWRONG_CURRENT_PUBLIC_KEY));

            let signer_capability_sig = multi_ed25519::new_signature_from_bytes(signer_capability_sig_bytes);
            assert!(multi_ed25519::signature_verify_strict_t(&signer_capability_sig, &pubkey, proof_challenge), error::invalid_argument(EINVALID_PROOF_OF_KNOWLEDGE));
        } else {
            abort error::invalid_argument(EINVALID_SCHEME)
        };

        // update the existing signer capability offer or put in a new signer capability offer for the current account
        option::swap_or_fill(&mut account_resource.signer_capability_offer.for, recipient_address);
    }

    public entry fun revoke_signer_capability(account: &signer, to_be_revoked_address: address) acquires Account {
        let addr = signer::address_of(account);
        assert!(exists_at(addr) && exists_at(to_be_revoked_address), error::not_found(EACCOUNT_DOES_NOT_EXIST));
        let account_resource = borrow_global_mut<Account>(addr);
        assert!(option::contains(&account_resource.signer_capability_offer.for, &to_be_revoked_address), error::not_found(ENO_SUCH_SIGNER_CAPABILITY));
        option::extract(&mut account_resource.signer_capability_offer.for);
    }

    /// Return a signer of the offerer, if there's an existing signer/rotation capability offer at the offerer's address
    public fun create_authorized_signer(account: &signer, offerer_address: address): signer acquires Account {
        assert!(exists_at(offerer_address), error::not_found(EACCOUNT_DOES_NOT_EXIST));

        // Check if there's an existing signer capability offer from the offerer
        let account_resource = borrow_global_mut<Account>(offerer_address);
        let addr = signer::address_of(account);
        assert!(option::contains(&account_resource.signer_capability_offer.for, &addr), error::not_found(ENO_SUCH_SIGNER_CAPABILITY));

        create_signer(offerer_address)
    }

    ///////////////////////////////////////////////////////////////////////////
    /// Basic account creation methods.
    ///////////////////////////////////////////////////////////////////////////

    /// This is a helper function to compute resource addresses. Computation of the address
    /// involves the use of a cryptographic hash operation and should be use thoughtfully.
    public fun create_resource_address(source: &address, seed: vector<u8>): address {
        let bytes = bcs::to_bytes(source);
        vector::append(&mut bytes, seed);
        from_bcs::to_address(hash::sha3_256(bytes))
    }

    /// A resource account is used to manage resources independent of an account managed by a user.
    public fun create_resource_account(source: &signer, seed: vector<u8>): (signer, SignerCapability) {
        let addr = create_resource_address(&signer::address_of(source), seed);
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
                addr == @0xa,
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

    public fun get_signer_capability_address(capability: &SignerCapability): address {
        capability.account
    }

    #[test(user = @0x1)]
    public entry fun test_create_resource_account(user: signer) {
        let (resource_account, resource_account_cap) = create_resource_account(&user, x"01");
        let resource_addr = signer::address_of(&resource_account);
        assert!(resource_addr != signer::address_of(&user), 0);
        assert!(resource_addr == get_signer_capability_address(&resource_account_cap), 1);
    }

    #[test_only]
    struct DummyResource has key {}

    #[test(user = @0x1)]
    public entry fun test_module_capability(user: signer) acquires DummyResource {
        let (resource_account, signer_cap) = create_resource_account(&user, x"01");
        assert!(signer::address_of(&resource_account) != signer::address_of(&user), 0);

        let resource_account_from_cap = create_signer_with_capability(&signer_cap);
        assert!(&resource_account == &resource_account_from_cap, 1);

        move_to(&resource_account_from_cap, DummyResource {});
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
        rotate_authentication_key(&alice, ED25519_SCHEME, pk, ED25519_SCHEME, pk, sig, sig);
    }

    #[test(alice = @0xa11ce)]
    #[expected_failure(abort_code = 262151)]
    public entry fun test_empty_signature(alice: signer) acquires Account, OriginatingAddress {
        create_account(signer::address_of(&alice));
        let test_signature = vector::empty<u8>();
        let pk = x"0000000000000000000000000000000000000000000000000000000000000000";
        rotate_authentication_key(&alice, ED25519_SCHEME, pk, ED25519_SCHEME, pk, test_signature, test_signature);
    }

    #[test_only]
    public entry fun create_account_from_ed25519_public_key(pk_bytes: vector<u8>): signer {
        let pk = ed25519::new_unvalidated_public_key_from_bytes(pk_bytes);
        let curr_auth_key = ed25519::unvalidated_public_key_to_authentication_key(&pk);
        let alice_address = from_bcs::to_address(curr_auth_key);
        let alice = create_account_unchecked(alice_address);
        alice
    }

    /*
    TODO bring back with generic rotation capability
        #[test(bob = @0x345)]
        #[expected_failure(abort_code = 65544)]
        public entry fun test_invalid_offer_rotation_capability(bob: signer) acquires Account {
            let pk = x"f66bf0ce5ceb582b93d6780820c2025b9967aedaa259bdbb9f3d0297eced0e18";
            let alice = create_account_from_ed25519_public_key(pk);
            create_account(signer::address_of(&bob));

            let invalid_signature = x"78f7d09ef7a9d8d7450d600b10231e6512610f919a63bd71bea1c907f7e101ed333bff360eeda97a8637a53fd622d597c03a0d6fd1315c6fa23719983ff7de0c";
            offer_rotation_capability_ed25519(&alice, invalid_signature, pk, signer::address_of(&bob));
        }
    */

    #[test(bob = @0x345)]
    #[expected_failure(abort_code = 65544)]
    public entry fun test_invalid_offer_signer_capability(bob: signer) acquires Account {
        // pk and signature are generated by sending a transaction in Rust and printing out the values
        let pk = x"f66bf0ce5ceb582b93d6780820c2025b9967aedaa259bdbb9f3d0297eced0e18";
        let alice = create_account_from_ed25519_public_key(pk);
        create_account(signer::address_of(&bob));

        let pk = x"f66bf0ce5ceb582b93d6780820c2025b9967aedaa259bdbb9f3d0297eced0e18";
        let invalid_signature = x"78f7d09ef7a9d8d7450d600b10231e6512610f919a63bd71bea1c907f7e101ed333bff360eeda97a8637a53fd622d597c03a0d6fd1315c6fa23719983ff7de0c";
        offer_signer_capability(&alice, invalid_signature, 0, pk, signer::address_of(&bob));
    }

    #[test(bob = @0x345)]
    public entry fun test_valid_check_signer_capability_and_create_authorized_signer(bob: signer) acquires Account {
        let pk = x"f66bf0ce5ceb582b93d6780820c2025b9967aedaa259bdbb9f3d0297eced0e18";
        let alice = create_account_from_ed25519_public_key(pk);
        create_account(signer::address_of(&bob));

        let valid_signature = x"cd181d65eb31193dcf1627fc0cc04208f66e7f243facc840830eaa458b176de570f73b661c127d98bc276c5a07ab242734b4d656163a86803561c0b9d9d01d0c";
        offer_signer_capability(&alice, valid_signature, 0, pk, signer::address_of(&bob));

        let alice_account_resource = borrow_global_mut<Account>(signer::address_of(&alice));
        assert!(option::contains(&alice_account_resource.signer_capability_offer.for, &signer::address_of(&bob)), 0);

        let signer = create_authorized_signer(&bob, signer::address_of(&alice));
        assert!(signer::address_of(&signer) == signer::address_of(&alice), 0);
    }

    #[test(bob = @0x345, charlie = @0x567)]
    #[expected_failure(abort_code = 393230)]
    public entry fun test_invalid_check_signer_capability_and_create_authorized_signer(bob: signer, charlie: signer) acquires Account {
        let pk = x"f66bf0ce5ceb582b93d6780820c2025b9967aedaa259bdbb9f3d0297eced0e18";
        let alice = create_account_from_ed25519_public_key(pk);
        create_account(signer::address_of(&bob));

        let valid_signature = x"cd181d65eb31193dcf1627fc0cc04208f66e7f243facc840830eaa458b176de570f73b661c127d98bc276c5a07ab242734b4d656163a86803561c0b9d9d01d0c";
        offer_signer_capability(&alice, valid_signature, 0, pk, signer::address_of(&bob));

        let alice_account_resource = borrow_global_mut<Account>(signer::address_of(&alice));
        assert!(option::contains(&alice_account_resource.signer_capability_offer.for, &signer::address_of(&bob)), 0);

        create_authorized_signer(&charlie, signer::address_of(&alice));
    }

    #[test(bob = @0x345)]
    public entry fun test_valid_revoke_signer_capability(bob: signer) acquires Account {
        let pk = x"f66bf0ce5ceb582b93d6780820c2025b9967aedaa259bdbb9f3d0297eced0e18";
        let alice = create_account_from_ed25519_public_key(pk);
        create_account(signer::address_of(&bob));

        let valid_signature = x"cd181d65eb31193dcf1627fc0cc04208f66e7f243facc840830eaa458b176de570f73b661c127d98bc276c5a07ab242734b4d656163a86803561c0b9d9d01d0c";
        offer_signer_capability(&alice, valid_signature, 0, pk, signer::address_of(&bob));
        revoke_signer_capability(&alice, signer::address_of(&bob));
    }

    #[test(bob = @0x345, charlie = @0x567)]
    #[expected_failure(abort_code = 393230)]
    public entry fun test_invalid_revoke_signer_capability(bob: signer, charlie: signer) acquires Account {
        let pk = x"f66bf0ce5ceb582b93d6780820c2025b9967aedaa259bdbb9f3d0297eced0e18";
        let alice = create_account_from_ed25519_public_key(pk);
        create_account(signer::address_of(&bob));
        create_account(signer::address_of(&charlie));

        let valid_signature = x"cd181d65eb31193dcf1627fc0cc04208f66e7f243facc840830eaa458b176de570f73b661c127d98bc276c5a07ab242734b4d656163a86803561c0b9d9d01d0c";
        offer_signer_capability(&alice, valid_signature, 0, pk, signer::address_of(&bob));
        revoke_signer_capability(&alice, signer::address_of(&charlie));
    }

    #[test(account = @aptos_framework)]
    public entry fun test_valid_rotate_authentication_key_multi_ed25519_to_multi_ed25519(account: signer) acquires Account, OriginatingAddress {
        initialize(&account);
        let curr_pk_bytes = x"26885bfb2e41746ffce3ab6ee9b5c9d15f4fbcba241362e814650a961daabb3464a19c23cf3b21ddda4b1370d2cf38aebd33732628049c7a00b97d4baf5b221d5ac38ef40f3fc7e8cd5c72229c0427b5fe68f580dbb9e297b544613e6948539e0328f304d79e0796c50344562a4b017019423b3dd6784495bada7e096cb9302f653e9618cf7b063de024db9adfb9d0b77dbb0048d312f167146f2807d991499fe21702f0df26ea83d418ccf8fd5fb6611c0cfb87c01e680e527a0b3fd7c0ac9b6e742db658fda20f93b446ac4b011430f8d455d71e29df2dca88a772bd598d5953f9fab64b9c5f7e0f424cd8659fb033aee099abeab7ce21463a8d4f84b803cd970ad8812196438311a72fc30720f42f863234ad30ba335d91281bf5c2c09e74e4c37b2f8d18781735e1301c0bd24d8b8e564a1e380cd862e4d058b99645efc98f27e1857f38ede528deb50f9310b2ee68756bac74279990de5dfd3557dbb0e631dc83e71e51abfa928bffff19b5e8ec751073fb371e3444272899cef16789a8812c21d54a4ae7fedd80623e2ed80f7652aa6a6ed97813d69c7f636b4a832088ed2d981e809dddf25566b926cb1677ba80793b4aa70087ef27c8b54d4c28f15ad2fe702293e628888fc548851063b89bcd1b0d8547a2856dbd9814bbff331e2e5a6ea7bb9cd6881ea498f46b3552d4815fed985b8c94c7800355f334b0e81fa7e3eb8ed19d83ac19d14d1bf0c4e978f339818514e36e9253172d8cb8ae4b194c2b507cb498e2c15fa567d17ef0be9d8cfd9eb17d46e71c0741bc81def63b6ff5559e83cd0a8f616ffa0b7c0f5b4c874d95ec92e97e7711b85cb2fef5a6f4fd4902";
        let new_pk_bytes = x"a63203808ff99ba9ea9133d4e1dafc42109c36ce4de07920ee6cae6136f97719cffa6ecd1582144a41c4388fa20a955339f96834c3901d98e7263ffe455266d1e14e07851e068243e3953bcc3ded766056f7a9586a1bd17fbaefc60c39c8e966a931455983b2b2004101bd7dabab7f5df17fd9281bee392cc7d65bff7f2b7730899c517c0035a971527f9e23d349c5e002e6208465b84a93bbdfb33f670ba4d69ac72f7bae9e577db6480ffef0559839c0317a00cb9cc7577fd18268dbf3b67f68b4f8a1cacd3fbd226e61fa2e849da1ed1f1d131a6b598957a022bdb462254871bd7dc490ac7675b638c1e750b1c7f6f035681abcca48b6b4f96367e4c75c8cd4da5321ec31899cb036469d478e028b5173b78336fc390be06b6a811d8d1022182fb3e8a4a94fd62267be02734b685beb24cf59bd1081216b11404a185df033aa8f546fd2fefd182dc364d48223d462dd8e4e1fa29889b526f67984b4862744ef7282cd606ff2aee79b05c4d366c7280dc92ade0415a3711a09d2d760a00e9f4960dcb1eb71fee22489f54cba68838816e6a0b3e83e85433ee836da1e4738229ec1ca8ad64c675fffbf0bc26022a39ddedae215286427a5bb1b84ca280d83dbe6ecef2dadceef0cd12347393f787e1e5e1b26f7a4fcfbae768e0e4f538141109ca98def3c85a8f7e54fc96e4547626a7a46536710a0c341ca4196676797a8590f";
        let cap_rotate_key = x"ddf094fd2039546fe229cc86b5c664aeaa7b40670aec18232e61eb591474ba33bccc63af9a5efe0563d03e11025b87bd820a5d8c72c0ac15ea1ff5632acb160808bb16b4088b9f985a3133c194bf67c2be4c34f788a857b26d5dd6c2ebb69bfcf959f44fbb2a96bd88da0091fe38eb029225ef3239e316372a35e08c5f10c70dc0000000";
        let cap_update_table = x"26dd17b80c92c5a1cbacf79edc1b46ff55dcba108559b14385dc3f91166d6194347ebcff7d1b1bf6864b342865c3a43e88b8203003b9a668b13921abbc6cad071479d0290c3639527d061b71596e0cbab07603f7cc69e6b1fd2ed7215d464c886e81e2bdf750a868180faea77b727b3f001b77b8cd8beb903a1b0a4228f01706870db6464656537e35a0a4584c9838cce37b14f97b81161239b48a1c596c2dc3e0177b3b9882b801a6cec5411346b4309f4ee6ab539ae0ffe0a40c678960f00c28c12fd8404e701115bcee7d07036cf8baae12869c4242a354836c82fa890c2bd396df231609e376d7faf0c31d5f6bb4d3b887a316f08a79b8e1054ed92a860b51c15b6bb53ffe515c22fff0638fc866320d0fce21a53776318b424a9dd2003efa3e8c3c142af50d556ae0c719cb0cca616128068b7b9671b53d7747ed4c870d76e00f8a32595fb04b31590271c34feac567596355ac13ce3bb784c34e81235fa021127e6395c35f3dd67134a1f78105982a923e259319114b04d060b2fdeb07d34bc50ec89f0ba7e749b164970449f95a5b865a3c6cd32e2bfc7f53ae02a3439f5f0259cdfa84eecba7f97302eca29ca33b89e4d8ba328017bbb6450ce1e70f426dcefec406ab7ad64f0ba6c0d8eeb273b1098ea2b2139b5828edfb847a68dadf43b956777c756152fca8d5c14cd66aa2a6e1c9d28d873a2bb118044e35c30cc9d21454821a516691cd301cedfc53a5ebb13310644dcd7ba1ac899cb8c895b8f9fdcd08b5b1096e338a5a92ff379a6395331a0629d97479231d186b418ed406e22974f3e0db1e4c8320427650f5048b7a2ffb6c9c168bf0e268e9915a42e15bb9c659f6fdf130fd2a5a8b841dde53920916b4c3fef07c692b583d41ea75a20cf7d77ab710816adf07a5773b3cd08ff6a95d5ef1ba036909e3c90801ec78b37bca8af036fd712684385a192e535aace4ea5871bfd32433184c48a51ea8491c05380ae41117d3a5b7e8480604087aa23c3f0fce2eda25da006c2ea181ff0efc0097d9bab5c95d6129c63fdb28faea20d76e7c52d74df81ac3009f1107d64dbc0406cc7d3fee03d9b77b06509d1be0a73cee18cbd0fbe9551de100e5eb8831c1e0b122f846de988076d9db22a600225454e877307692d0e8e9eea1341d48a5720163120187a2264d671299fd5a28f52a9bee534008fa4a0390238c9ae426507218f903acedd135708a33092c2e2bddb671237fd2b3cf434a1d08a8596ae215fe02b008fd2932940a2e925e923e04d20e8f8d9a6f40ce0eb550687885fcdecc98ddbd54d6b73217fc212a480841820e0d67894edb84538c2a0d4ae342868194d204fffe0000";

        let curr_pk = multi_ed25519::new_unvalidated_public_key_from_bytes(curr_pk_bytes);
        let curr_auth_key = multi_ed25519::unvalidated_public_key_to_authentication_key(&curr_pk);
        let alice_address = from_bcs::to_address(curr_auth_key);
        let alice = create_account_unchecked(alice_address);

        rotate_authentication_key(&alice, MULTI_ED25519_SCHEME, curr_pk_bytes, MULTI_ED25519_SCHEME, new_pk_bytes, cap_rotate_key, cap_update_table);
        let address_map = &mut borrow_global_mut<OriginatingAddress>(@aptos_framework).address_map;
        let new_pk = multi_ed25519::new_unvalidated_public_key_from_bytes(new_pk_bytes);
        let new_auth_key = multi_ed25519::unvalidated_public_key_to_authentication_key(&new_pk);
        let new_address = from_bcs::to_address(new_auth_key);
        let expected_originating_address = table::borrow(address_map, new_address);
        assert!(*expected_originating_address == alice_address, 0);

        let account_resource = borrow_global_mut<Account>(alice_address);
        assert!(account_resource.authentication_key == new_auth_key, 0);
    }

    #[test(account = @aptos_framework)]
    public entry fun test_valid_rotate_authentication_key_multi_ed25519_to_ed25519(account: signer) acquires Account, OriginatingAddress {
        initialize(&account);
        let curr_pk_bytes = x"26885bfb2e41746ffce3ab6ee9b5c9d15f4fbcba241362e814650a961daabb3464a19c23cf3b21ddda4b1370d2cf38aebd33732628049c7a00b97d4baf5b221d5ac38ef40f3fc7e8cd5c72229c0427b5fe68f580dbb9e297b544613e6948539e0328f304d79e0796c50344562a4b017019423b3dd6784495bada7e096cb9302f653e9618cf7b063de024db9adfb9d0b77dbb0048d312f167146f2807d991499fe21702f0df26ea83d418ccf8fd5fb6611c0cfb87c01e680e527a0b3fd7c0ac9b6e742db658fda20f93b446ac4b011430f8d455d71e29df2dca88a772bd598d5953f9fab64b9c5f7e0f424cd8659fb033aee099abeab7ce21463a8d4f84b803cd970ad8812196438311a72fc30720f42f863234ad30ba335d91281bf5c2c09e74e4c37b2f8d18781735e1301c0bd24d8b8e564a1e380cd862e4d058b99645efc98f27e1857f38ede528deb50f9310b2ee68756bac74279990de5dfd3557dbb0e631dc83e71e51abfa928bffff19b5e8ec751073fb371e3444272899cef16789a8812c21d54a4ae7fedd80623e2ed80f7652aa6a6ed97813d69c7f636b4a832088ed2d981e809dddf25566b926cb1677ba80793b4aa70087ef27c8b54d4c28f15ad2fe702293e628888fc548851063b89bcd1b0d8547a2856dbd9814bbff331e2e5a6ea7bb9cd6881ea498f46b3552d4815fed985b8c94c7800355f334b0e81fa7e3eb8ed19d83ac19d14d1bf0c4e978f339818514e36e9253172d8cb8ae4b194c2b507cb498e2c15fa567d17ef0be9d8cfd9eb17d46e71c0741bc81def63b6ff5559e83cd0a8f616ffa0b7c0f5b4c874d95ec92e97e7711b85cb2fef5a6f4fd4902";
        let new_pk_bytes = x"20fdbac9b10b7587bba7b5bc163bce69e796d71e4ed44c10fcb4488689f7a144";
        let cap_rotate_key = x"0bc503a99ee09a2bfaeb0039a092abda54cf7493608c01a2e0ac4a0c49958fcbf7eb0521e388ec73b03b978dce79ffda20194aca52cdd13f35c4776de8d27808f0d8c0dbeb14700b46e3c927d848aeba74e0749cdc6429fa1aba1d3e7ef57948bef0810125ccaa2de25a167d13f5725bbc85fcac1b03dff944275d4b4cad3c0ac0000000";
        let cap_update_table = x"dcb63645f22c9c3f9ff6b05293dc3c0e22e4bd6d6c4001d68869139e78a645d4c0745b61538916b0f6e42736f0dbba19dbd6d1eee5bdd5ef3e7c1d0617b72d01";

        let curr_pk = multi_ed25519::new_unvalidated_public_key_from_bytes(curr_pk_bytes);
        let curr_auth_key = multi_ed25519::unvalidated_public_key_to_authentication_key(&curr_pk);
        let alice_address = from_bcs::to_address(curr_auth_key);
        let alice = create_account_unchecked(alice_address);

        rotate_authentication_key(&alice, MULTI_ED25519_SCHEME, curr_pk_bytes, ED25519_SCHEME, new_pk_bytes, cap_rotate_key, cap_update_table);
        let address_map = &mut borrow_global_mut<OriginatingAddress>(@aptos_framework).address_map;
        let new_pk = ed25519::new_unvalidated_public_key_from_bytes(new_pk_bytes);
        let new_auth_key = ed25519::unvalidated_public_key_to_authentication_key(&new_pk);
        let new_address = from_bcs::to_address(new_auth_key);
        let expected_originating_address = table::borrow(address_map, new_address);
        assert!(*expected_originating_address == alice_address, 0);

        let account_resource = borrow_global_mut<Account>(alice_address);
        assert!(account_resource.authentication_key == new_auth_key, 0);
    }
}
