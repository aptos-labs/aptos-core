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

    struct SignerCapabilityOfferProofChallengeV2 has copy, drop {
        sequence_number: u64,
        source_address: address,
        recipient_address: address,
    }

    const MAX_U64: u128 = 18446744073709551615;
    const ZERO_AUTH_KEY: vector<u8> = x"0000000000000000000000000000000000000000000000000000000000000000";

    /// Scheme identifier for Ed25519 signatures used to derive authentication keys for Ed25519 public keys.
    const ED25519_SCHEME: u8 = 0;
    /// Scheme identifier for MultiEd25519 signatures used to derive authentication keys for MultiEd25519 public keys.
    const MULTI_ED25519_SCHEME: u8 = 1;
    /// Scheme identifier used when hashing an account's address together with a seed to derive the address (not the
    /// authentication key) of a resource account. This is an abuse of the notion of a scheme identifier which, for now,
    /// serves to domain separate hashes used to derive resource account addresses from hashes used to derive
    /// authentication keys. Without such separation, an adversary could create (and get a signer for) a resource account
    /// whose address matches an existing address of a MultiEd25519 wallet.
    const DERIVE_RESOURCE_ACCOUNT_SCHEME: u8 = 255;

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
    /// An attempt to create a resource account on a claimed account
    const ERESOURCE_ACCCOUNT_EXISTS: u64 = 15;
    /// An attempt to create a resource account on an account that has a committed transaction
    const EACCOUNT_ALREADY_USED: u64 = 16;

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

        // NOTE: @core_resources gets created via a `create_account` call, so we do not include it below.
        assert!(
            new_address != @vm_reserved && new_address != @aptos_framework && new_address != @aptos_token,
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
        let sequence_number = &mut borrow_global_mut<Account>(addr).sequence_number;

        assert!(
            (*sequence_number as u128) < MAX_U64,
            error::out_of_range(ESEQUENCE_NUMBER_TOO_BIG)
        );

        *sequence_number = *sequence_number + 1;
    }

    public fun get_authentication_key(addr: address): vector<u8> acquires Account {
        *&borrow_global<Account>(addr).authentication_key
    }

    public(friend) fun rotate_authentication_key_internal(account: &signer, new_auth_key: vector<u8>) acquires Account {
        let addr = signer::address_of(account);
        assert!(exists_at(addr), error::not_found(EACCOUNT_DOES_NOT_EXIST));
        assert!(
            vector::length(&new_auth_key) == 32,
            error::invalid_argument(EMALFORMED_AUTHENTICATION_KEY)
        );
        let account_resource = borrow_global_mut<Account>(addr);
        account_resource.authentication_key = new_auth_key;
    }

    fun assert_valid_signature_and_get_auth_key(scheme: u8, public_key_bytes: vector<u8>, signature: vector<u8>, challenge: &RotationProofChallenge): vector<u8> {
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
    /// To authorize the rotation, we need two signatures:
    /// - the first signature `cap_rotate_key` refers to the signature by the account owner's current key on a valid `RotationProofChallenge`,
    /// demonstrating that the user intends to and has the capability to rotate the authentication key of this account;
    /// - the second signature `cap_update_table` refers to the signature by the new key (that the account owner wants to rotate to) on a
    /// valid `RotationProofChallenge`, demonstrating that the user owns the new private key, and has the authority to update the
    /// `OriginatingAddress` map with the new address mapping <new_address, originating_address>.
    /// To verify signatures, we need their corresponding public key and public key scheme: we use `from_scheme` and `from_public_key_bytes`
    /// to verify `cap_rotate_key`, and `to_scheme` and `to_public_key_bytes` to verify `cap_update_table`.
    /// A scheme of 0 refers to an Ed25519 key and a scheme of 1 refers to Multi-Ed25519 keys.
    /// `originating address` refers to an account's original/first address.
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

        // Verify the given `from_public_key_bytes` matches this account's current authentication key.
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

        // Construct a valid `RotationProofChallenge` that `cap_rotate_key` and `cap_update_table` will validate against.
        let curr_auth_key_as_address = from_bcs::to_address(account_resource.authentication_key);
        let challenge = RotationProofChallenge {
            sequence_number: account_resource.sequence_number,
            originator: addr,
            current_auth_key: curr_auth_key_as_address,
            new_public_key: to_public_key_bytes,
        };

        // Assert the challenges signed by the current and new keys are valid
        let curr_auth_key = assert_valid_signature_and_get_auth_key(from_scheme, from_public_key_bytes, cap_rotate_key, &challenge);
        let new_auth_key = assert_valid_signature_and_get_auth_key(to_scheme, to_public_key_bytes, cap_update_table, &challenge);

        // Update the `OriginatingAddress` table, so that we can find the originating address using the latest address
        // in the event of key recovery
        let address_map = &mut borrow_global_mut<OriginatingAddress>(@aptos_framework).address_map;
        let new_auth_key_as_address = from_bcs::to_address(new_auth_key);
        if (table::contains(address_map, curr_auth_key_as_address)) {
            // Assert that we're calling from the account with the originating address.
            // For example, if we have already rotated from keypair_a to keypair_b, and are trying to rotate from
            // keypair_b to keypair_c, we could call `rotate_authentication_key` from address_a or address_b.
            // Here, we wanted to enforce the standard that we expect the call to come from the signer with address a.
            // If a signer with address b calls this function with two valid signatures, it will abort at this step,
            // because address b is not the account's originating address.
            // This means that after key rotation, the account's address should be the same, but their public key
            // and private key should be updated to the new ones.
            assert!(addr == table::remove(address_map, curr_auth_key_as_address), error::not_found(EINVALID_ORIGINATING_ADDRESS));
        };
        table::add(address_map, new_auth_key_as_address, addr);

        event::emit_event<KeyRotationEvent>(
            &mut account_resource.key_rotation_events,
            KeyRotationEvent {
                old_authentication_key: curr_auth_key,
                new_authentication_key: new_auth_key,
            }
        );

        account_resource.authentication_key = new_auth_key;
    }

    /// Offers signer capability on behalf of `account` to the account at address `recipient_address`.
    /// An account can delegate its signer capability to only one other address at one time.
    /// `signer_capability_key_bytes` is the `SignerCapabilityOfferProofChallengeV2` signed by the account owner's key
    /// `account_scheme` is the scheme of the account (ed25519 or multi_ed25519).
    /// `account_public_key_bytes` is the public key of the account owner.
    /// `recipient_address` is the address of the recipient of the signer capability - note that if there's an existing
    /// `recipient_address` in the account owner's `SignerCapabilityOffer`, this will replace the
    /// previous `recipient_address` upon successful verification (the previous recipient will no longer have access
    /// to the account owner's signer capability).
    public entry fun offer_signer_capability(
        account: &signer,
        signer_capability_sig_bytes: vector<u8>,
        account_scheme: u8,
        account_public_key_bytes: vector<u8>,
        recipient_address: address
    ) acquires Account {
        let source_address = signer::address_of(account);
        assert!(exists_at(recipient_address), error::not_found(EACCOUNT_DOES_NOT_EXIST));

        let account_resource = borrow_global_mut<Account>(source_address);

        // Proof that this account intends to delegate its signer capability to another account.
        let proof_challenge = SignerCapabilityOfferProofChallengeV2 {
            sequence_number: account_resource.sequence_number,
            source_address,
            recipient_address,
        };

        // Verify that the `SignerCapabilityOfferProofChallengeV2` has the right information and is signed by the account owner's key
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

        // Update the existing signer capability offer or put in a new signer capability offer for the recipient.
        option::swap_or_fill(&mut account_resource.signer_capability_offer.for, recipient_address);
    }

    /// Revoke the account owner's signer capability offer for `to_be_revoked_address` (i.e., the address that
    /// has a signer capability offer from `account` but will be revoked in this function).
    public entry fun revoke_signer_capability(account: &signer, to_be_revoked_address: address) acquires Account {
        assert!(exists_at(to_be_revoked_address), error::not_found(EACCOUNT_DOES_NOT_EXIST));
        let addr = signer::address_of(account);
        let account_resource = borrow_global_mut<Account>(addr);
        assert!(option::contains(&account_resource.signer_capability_offer.for, &to_be_revoked_address), error::not_found(ENO_SUCH_SIGNER_CAPABILITY));
        option::extract(&mut account_resource.signer_capability_offer.for);
    }

    /// Return an authorized signer of the offerer, if there's an existing signer capability offer for `account`
    /// at the offerer's address.
    public fun create_authorized_signer(account: &signer, offerer_address: address): signer acquires Account {
        assert!(exists_at(offerer_address), error::not_found(EACCOUNT_DOES_NOT_EXIST));

        // Check if there's an existing signer capability offer from the offerer.
        let account_resource = borrow_global<Account>(offerer_address);
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
        vector::push_back(&mut bytes, DERIVE_RESOURCE_ACCOUNT_SCHEME);
        from_bcs::to_address(hash::sha3_256(bytes))
    }

    /// A resource account is used to manage resources independent of an account managed by a user.
    /// In Aptos a resource account is created based upon the sha3 256 of the source's address and additional seed data.
    /// A resource account can only be created once, this is designated by setting the
    /// `Account::signer_capability_offer::for` to the address of the resource account. While an entity may call
    /// `create_account` to attempt to claim an account ahead of the creation of a resource account, if found Aptos will
    /// transition ownership of the account over to the resource account. This is done by validating that the account has
    /// yet to execute any transactions and that the `Account::signer_capability_offer::for` is none. The probability of a
    /// collision where someone has legitimately produced a private key that maps to a resource account address is less
    /// than `(1/2)^(256)`.
    public fun create_resource_account(source: &signer, seed: vector<u8>): (signer, SignerCapability) acquires Account {
        let resource_addr = create_resource_address(&signer::address_of(source), seed);
        let resource = if (exists_at(resource_addr)) {
            let account = borrow_global<Account>(resource_addr);
            assert!(
                option::is_none(&account.signer_capability_offer.for),
                error::already_exists(ERESOURCE_ACCCOUNT_EXISTS),
            );
            assert!(
                account.sequence_number == 0,
                error::invalid_state(EACCOUNT_ALREADY_USED),
            );
            create_signer(resource_addr)
        } else {
            create_account_unchecked(resource_addr)
        };

        // By default, only the SignerCapability should have control over the resource account and not the auth key.
        // If the source account wants direct control via auth key, they would need to explicitly rotate the auth key
        // of the resource account using the SignerCapability.
        rotate_authentication_key_internal(&resource, ZERO_AUTH_KEY);

        let account = borrow_global_mut<Account>(resource_addr);
        account.signer_capability_offer.for = option::some(resource_addr);
        let signer_cap = SignerCapability { account: resource_addr };
        (resource, signer_cap)
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
    public entry fun test_create_resource_account(user: signer) acquires Account {
        let (resource_account, resource_account_cap) = create_resource_account(&user, x"01");
        let resource_addr = signer::address_of(&resource_account);
        assert!(resource_addr != signer::address_of(&user), 0);
        assert!(resource_addr == get_signer_capability_address(&resource_account_cap), 1);
    }

    #[test]
    #[expected_failure(abort_code = 0x10007)]
    public entry fun test_cannot_control_resource_account_via_auth_key() acquires Account {
        let alice_pk = x"4141414141414141414141414141414141414141414141414141414141414145";
        let alice = create_account_from_ed25519_public_key(alice_pk);
        let alice_auth = get_authentication_key(signer::address_of(&alice)); // must look like a valid public key

        let (eve_sk, eve_pk) = ed25519::generate_keys();
        let eve_pk_bytes = ed25519::validated_public_key_to_bytes(&eve_pk);
        let eve = create_account_from_ed25519_public_key(eve_pk_bytes);
        let recipient_address = signer::address_of(&eve);

        let seed = *&eve_pk_bytes; // multisig public key
        vector::push_back(&mut seed, 1); // multisig threshold
        vector::push_back(&mut seed, 1); // signature scheme id
        let (resource, _) = create_resource_account(&alice, seed);

        let resource_addr = signer::address_of(&resource);
        let proof_challenge = SignerCapabilityOfferProofChallengeV2 {
            sequence_number: borrow_global_mut<Account>(resource_addr).sequence_number,
            source_address: resource_addr,
            recipient_address,
        };

        let eve_sig = ed25519::sign_struct(&eve_sk, copy proof_challenge);

        // Construct a malicious 1-out-of-2 multisig PK over Alice's authentication key and Eve's Ed25519 PK.
        let account_public_key_bytes = alice_auth;
        vector::append(&mut account_public_key_bytes, *&eve_pk_bytes);
        vector::push_back(&mut account_public_key_bytes, 1); // Multisig verification threshold.
        let fake_pk = multi_ed25519::new_unvalidated_public_key_from_bytes(account_public_key_bytes);

        // Construct a multisig for `proof_challenge` as if it is signed by the signers behind `fake_pk`,
        // Eve being the only participant.
        let signer_capability_sig_bytes = x"";
        vector::append(&mut signer_capability_sig_bytes, ed25519::signature_to_bytes(&eve_sig));
        vector::append(&mut signer_capability_sig_bytes, x"40000000"); // Signers bitmap.
        let fake_sig = multi_ed25519::new_signature_from_bytes(signer_capability_sig_bytes);

        assert!(multi_ed25519::signature_verify_strict_t(&fake_sig, &fake_pk, proof_challenge), error::invalid_state(EINVALID_PROOF_OF_KNOWLEDGE));
        offer_signer_capability(&resource, signer_capability_sig_bytes, MULTI_ED25519_SCHEME, account_public_key_bytes, recipient_address);
    }

    #[test_only]
    struct DummyResource has key {}

    #[test(user = @0x1)]
    public entry fun test_module_capability(user: signer) acquires Account, DummyResource {
        let (resource_account, signer_cap) = create_resource_account(&user, x"01");
        assert!(signer::address_of(&resource_account) != signer::address_of(&user), 0);

        let resource_account_from_cap = create_signer_with_capability(&signer_cap);
        assert!(&resource_account == &resource_account_from_cap, 1);

        move_to(&resource_account_from_cap, DummyResource {});
        borrow_global<DummyResource>(signer::address_of(&resource_account));
    }

    #[test(user = @0x1)]
    public entry fun test_resource_account_and_create_account(user: signer) acquires Account {
        let resource_addr = create_resource_address(&@0x1, x"01");
        create_account_unchecked(resource_addr);

        create_resource_account(&user, x"01");
    }

    #[test(user = @0x1)]
    #[expected_failure(abort_code = 0x8000f)]
    public entry fun test_duplice_create_resource_account(user: signer) acquires Account {
        create_resource_account(&user, x"01");
        create_resource_account(&user, x"01");
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

    //
    // Tests for offering & revoking signer capabilities
    //

    #[test(bob = @0x345)]
    #[expected_failure(abort_code = 65544)]
    public entry fun test_invalid_offer_signer_capability(bob: signer) acquires Account {
        let (_alice_sk, alice_pk) = ed25519::generate_keys();
        let alice_pk_bytes = ed25519::validated_public_key_to_bytes(&alice_pk);
        let alice = create_account_from_ed25519_public_key(alice_pk_bytes);
        let alice_addr = signer::address_of(&alice);

        let bob_addr = signer::address_of(&bob);
        create_account(bob_addr);

        let challenge = SignerCapabilityOfferProofChallengeV2 {
            sequence_number: borrow_global<Account>(alice_addr).sequence_number,
            source_address: alice_addr,
            recipient_address: bob_addr,
        };

        let sig = ed25519::sign_struct(&_alice_sk, challenge);

        // Maul the signature and make sure the call would fail
        let invalid_signature = ed25519::signature_to_bytes(&sig);
        let first_sig_byte = vector::borrow_mut(&mut invalid_signature, 0);
        *first_sig_byte = *first_sig_byte + 1;

        offer_signer_capability(&alice, invalid_signature, 0, alice_pk_bytes, bob_addr);
    }

    #[test(bob = @0x345)]
    public entry fun test_valid_check_signer_capability_and_create_authorized_signer(bob: signer) acquires Account {
        let (alice_sk, alice_pk) = ed25519::generate_keys();
        let alice_pk_bytes = ed25519::validated_public_key_to_bytes(&alice_pk);
        let alice = create_account_from_ed25519_public_key(alice_pk_bytes);
        let alice_addr = signer::address_of(&alice);

        let bob_addr = signer::address_of(&bob);
        create_account(bob_addr);

        let challenge = SignerCapabilityOfferProofChallengeV2 {
            sequence_number: borrow_global<Account>(alice_addr).sequence_number,
            source_address: alice_addr,
            recipient_address: bob_addr,
        };

        let alice_signer_capability_offer_sig = ed25519::sign_struct(&alice_sk, challenge);

        offer_signer_capability(&alice, ed25519::signature_to_bytes(&alice_signer_capability_offer_sig), 0, alice_pk_bytes, bob_addr);

        assert!(option::contains(&borrow_global<Account>(alice_addr).signer_capability_offer.for, &bob_addr), 0);

        let signer = create_authorized_signer(&bob, alice_addr);
        assert!(signer::address_of(&signer) == signer::address_of(&alice), 0);
    }

    #[test(bob = @0x345, charlie = @0x567)]
    #[expected_failure(abort_code = 393230)]
    public entry fun test_invalid_check_signer_capability_and_create_authorized_signer(bob: signer, charlie: signer) acquires Account {
        let (alice_sk, alice_pk) = ed25519::generate_keys();
        let alice_pk_bytes = ed25519::validated_public_key_to_bytes(&alice_pk);
        let alice = create_account_from_ed25519_public_key(alice_pk_bytes);
        let alice_addr = signer::address_of(&alice);

        let bob_addr = signer::address_of(&bob);
        create_account(bob_addr);

        let challenge = SignerCapabilityOfferProofChallengeV2 {
            sequence_number: borrow_global<Account>(alice_addr).sequence_number,
            source_address: alice_addr,
            recipient_address: bob_addr,
        };

        let alice_signer_capability_offer_sig = ed25519::sign_struct(&alice_sk, challenge);

        offer_signer_capability(&alice, ed25519::signature_to_bytes(&alice_signer_capability_offer_sig), 0, alice_pk_bytes, bob_addr);

        let alice_account_resource = borrow_global_mut<Account>(alice_addr);
        assert!(option::contains(&alice_account_resource.signer_capability_offer.for, &bob_addr), 0);

        create_authorized_signer(&charlie, alice_addr);
    }

    #[test(bob = @0x345)]
    public entry fun test_valid_revoke_signer_capability(bob: signer) acquires Account {
        let (alice_sk, alice_pk) = ed25519::generate_keys();
        let alice_pk_bytes = ed25519::validated_public_key_to_bytes(&alice_pk);
        let alice = create_account_from_ed25519_public_key(alice_pk_bytes);
        let alice_addr = signer::address_of(&alice);

        let bob_addr = signer::address_of(&bob);
        create_account(bob_addr);

        let challenge = SignerCapabilityOfferProofChallengeV2 {
            sequence_number: borrow_global<Account>(alice_addr).sequence_number,
            source_address: alice_addr,
            recipient_address: bob_addr,
        };

        let alice_signer_capability_offer_sig = ed25519::sign_struct(&alice_sk, challenge);

        offer_signer_capability(&alice, ed25519::signature_to_bytes(&alice_signer_capability_offer_sig), 0, alice_pk_bytes, bob_addr);
        revoke_signer_capability(&alice, bob_addr);
    }

    #[test(bob = @0x345, charlie = @0x567)]
    #[expected_failure(abort_code = 393230)]
    public entry fun test_invalid_revoke_signer_capability(bob: signer, charlie: signer) acquires Account {
        let (alice_sk, alice_pk) = ed25519::generate_keys();
        let alice_pk_bytes = ed25519::validated_public_key_to_bytes(&alice_pk);
        let alice = create_account_from_ed25519_public_key(alice_pk_bytes);
        let alice_addr = signer::address_of(&alice);
        let alice_account_resource = borrow_global<Account>(alice_addr);

        let bob_addr = signer::address_of(&bob);
        create_account(bob_addr);

        let charlie_addr = signer::address_of(&charlie);
        create_account(charlie_addr);

        let challenge = SignerCapabilityOfferProofChallengeV2 {
            sequence_number: alice_account_resource.sequence_number,
            source_address: alice_addr,
            recipient_address: bob_addr,
        };
        let alice_signer_capability_offer_sig = ed25519::sign_struct(&alice_sk, challenge);
        offer_signer_capability(&alice, ed25519::signature_to_bytes(&alice_signer_capability_offer_sig), 0, alice_pk_bytes, bob_addr);
        revoke_signer_capability(&alice, charlie_addr);
    }

    //
    // Tests for key rotation
    //

    #[test(account = @aptos_framework)]
    public entry fun test_valid_rotate_authentication_key_multi_ed25519_to_multi_ed25519(account: signer) acquires Account, OriginatingAddress {
        initialize(&account);
        let (curr_sk, curr_pk) = multi_ed25519::generate_keys(2, 3);
        let curr_pk_unvalidated = multi_ed25519::public_key_to_unvalidated(&curr_pk);
        let curr_auth_key = multi_ed25519::unvalidated_public_key_to_authentication_key(&curr_pk_unvalidated);
        let alice_addr = from_bcs::to_address(curr_auth_key);
        let alice = create_account_unchecked(alice_addr);

        let (new_sk, new_pk) = multi_ed25519::generate_keys(4, 5);
        let new_pk_unvalidated = multi_ed25519::public_key_to_unvalidated(&new_pk);
        let new_auth_key = multi_ed25519::unvalidated_public_key_to_authentication_key(&new_pk_unvalidated);
        let new_address = from_bcs::to_address(new_auth_key);

        let challenge = RotationProofChallenge {
            sequence_number: borrow_global<Account>(alice_addr).sequence_number,
            originator: alice_addr,
            current_auth_key: alice_addr,
            new_public_key: multi_ed25519::unvalidated_public_key_to_bytes(&new_pk_unvalidated),
        };

        let from_sig = multi_ed25519::sign_struct(&curr_sk, challenge);
        let to_sig = multi_ed25519::sign_struct(&new_sk, challenge);

        rotate_authentication_key(
            &alice,
            MULTI_ED25519_SCHEME,
            multi_ed25519::unvalidated_public_key_to_bytes(&curr_pk_unvalidated),
            MULTI_ED25519_SCHEME,
            multi_ed25519::unvalidated_public_key_to_bytes(&new_pk_unvalidated),
            multi_ed25519::signature_to_bytes(&from_sig),
            multi_ed25519::signature_to_bytes(&to_sig),
        );
        let address_map = &mut borrow_global_mut<OriginatingAddress>(@aptos_framework).address_map;
        let expected_originating_address = table::borrow(address_map, new_address);
        assert!(*expected_originating_address == alice_addr, 0);
        assert!(borrow_global<Account>(alice_addr).authentication_key == new_auth_key, 0);
    }

    #[test(account = @aptos_framework)]
    public entry fun test_valid_rotate_authentication_key_multi_ed25519_to_ed25519(account: signer) acquires Account, OriginatingAddress {
        initialize(&account);

        let (curr_sk, curr_pk) = multi_ed25519::generate_keys(2, 3);
        let curr_pk_unvalidated = multi_ed25519::public_key_to_unvalidated(&curr_pk);
        let curr_auth_key = multi_ed25519::unvalidated_public_key_to_authentication_key(&curr_pk_unvalidated);
        let alice_addr = from_bcs::to_address(curr_auth_key);
        let alice = create_account_unchecked(alice_addr);

        let account_resource = borrow_global_mut<Account>(alice_addr);

        let (new_sk, new_pk) = ed25519::generate_keys();
        let new_pk_unvalidated = ed25519::public_key_to_unvalidated(&new_pk);
        let new_auth_key = ed25519::unvalidated_public_key_to_authentication_key(&new_pk_unvalidated);
        let new_addr = from_bcs::to_address(new_auth_key);

        let challenge = RotationProofChallenge {
            sequence_number: account_resource.sequence_number,
            originator: alice_addr,
            current_auth_key: alice_addr,
            new_public_key: ed25519::unvalidated_public_key_to_bytes(&new_pk_unvalidated),
        };

        let from_sig = multi_ed25519::sign_struct(&curr_sk, challenge);
        let to_sig = ed25519::sign_struct(&new_sk, challenge);

        rotate_authentication_key(
            &alice,
            MULTI_ED25519_SCHEME,
            multi_ed25519::unvalidated_public_key_to_bytes(&curr_pk_unvalidated),
            ED25519_SCHEME,
            ed25519::unvalidated_public_key_to_bytes(&new_pk_unvalidated),
            multi_ed25519::signature_to_bytes(&from_sig),
            ed25519::signature_to_bytes(&to_sig),
        );

        let address_map = &mut borrow_global_mut<OriginatingAddress>(@aptos_framework).address_map;
        let expected_originating_address = table::borrow(address_map, new_addr);
        assert!(*expected_originating_address == alice_addr, 0);
        assert!(borrow_global<Account>(alice_addr).authentication_key == new_auth_key, 0);
    }
}
