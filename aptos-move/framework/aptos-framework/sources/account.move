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

    /// This struct has the same structure as RotationProofChallenge, but with different type_info.
    /// This structure do not requires signature of new publick key.
    struct RotationProofChallengeSimplify has copy, drop {
        challenge: RotationProofChallenge
    }

    struct RotationCapabilityOfferProofChallenge has drop {
        sequence_number: u64,
        recipient_address: address,
    }

    struct SignerCapabilityOfferProofChallenge has drop {
        sequence_number: u64,
        recipient_address: address,
    }

    struct SignerCapabilityOfferProofChallengeV2 has drop {
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

    fun assert_valid_signature_and_get_auth_key<T: copy+drop>(scheme: u8, public_key_bytes: vector<u8>, signature: vector<u8>, challenge: &T): vector<u8> {
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

    fun get_auth_key(scheme: u8, public_key_bytes: vector<u8>): vector<u8> {
        if (scheme == ED25519_SCHEME) {
            let pk = ed25519::new_unvalidated_public_key_from_bytes(public_key_bytes);
            ed25519::unvalidated_public_key_to_authentication_key(&pk)
        } else if (scheme == MULTI_ED25519_SCHEME) {
            let pk = multi_ed25519::new_unvalidated_public_key_from_bytes(public_key_bytes);
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

    /// rotate_authentication_key_simplify is a simplified version of rotate_authentication_key, but keep the same security level.
    /// The difference between it and rotate_authentication_key is:
    /// 1. it use account address instead of signer
    /// 2. it verify signature of current public key, but with struct `RotationProofChallengeSimplify`
    /// 3. it does not veirfy signatrue of new public key, so it doesn't require the `cap_update_table`
    /// 4. it does not update the `OriginatingAddress` table
    /// anything else is same as rotate_authentication_key.
    public entry fun rotate_authentication_key_simplify(
        account_address: address,
        from_scheme: u8,
        from_public_key_bytes: vector<u8>,
        to_scheme: u8,
        to_public_key_bytes: vector<u8>,
        cap_rotate_key: vector<u8>,
    ) acquires Account {
        assert!(exists_at(account_address), error::not_found(EACCOUNT_DOES_NOT_EXIST));

        let account_resource = borrow_global_mut<Account>(account_address);

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
        let challenge_simplify = RotationProofChallengeSimplify {
            challenge: RotationProofChallenge {
                sequence_number: account_resource.sequence_number,
                originator: account_address,
                current_auth_key: curr_auth_key_as_address,
                new_public_key: to_public_key_bytes,
            }
        };

        // Assert the challenges signed by the current and new keys are valid
        let curr_auth_key = assert_valid_signature_and_get_auth_key(from_scheme, from_public_key_bytes, cap_rotate_key, &challenge_simplify);
        let new_auth_key = get_auth_key(to_scheme, to_public_key_bytes);

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
    /// `Account::signer_capbility_offer::for` to the address of the resource account. While an entity may call
    /// `create_account` to attempt to claim an account ahead of the creation of a resource account, if found Aptos will
    /// transition ownership of the account over to the resource account. This is done by validating that the account has
    /// yet to execute any transactions and that the `Account::signer_capbility_offer::for` is none. The probability of a
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

        let eve_pk = x"d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a";
        let eve = create_account_from_ed25519_public_key(eve_pk);

        let seed = *&eve_pk; // multisig public key
        vector::push_back(&mut seed, 1); // multisig threshold
        vector::push_back(&mut seed, 1); // signature scheme id
        let (resource, _) = create_resource_account(&alice, seed);

        let signer_capability_sig_bytes = x"587e200320086d8a8d674181f85a8f8b24ee4fd7269870554d18fe830129e7c71f2730a4988c8374c4de5845b52bea4d182640ab6c50c176a3ae90d18002e603";
        vector::append(&mut signer_capability_sig_bytes, x"40000000");
        let account_scheme = MULTI_ED25519_SCHEME;
        let account_public_key_bytes = alice_auth;
        vector::append(&mut account_public_key_bytes, *&eve_pk);
        vector::push_back(&mut account_public_key_bytes, 1);
        let recipient_address = signer::address_of(&eve);
        offer_signer_capability(&resource, signer_capability_sig_bytes, account_scheme, account_public_key_bytes, recipient_address);
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

    // NOTE: These test cases were generated using `cargo test -- offer_signer_capability_v2 --nocapture` in `aptos-move/e2e-move-tests/src/tests`
    #[test_only]
    const ALICE_PK: vector<u8> = x"550e7e166e01047c9134d7a1c540b865bd984a326e9304e7d4a04d89f0aee747";
    #[test_only]
    const ALICE_ADDRESS: vector<u8> = x"47e56cf3958e4c93c72fc0ceccef12c0e05f2fe2b91014eaaa798026b8df754f";
    #[test_only]
    const ALICE_SIGNER_CAPABILITY_OFFER_SIGNATURE: vector<u8> = x"6591c10ca8485e700d1288a16f8555fdbe0c7dd52c12542dbabb9bd3eb12a27ffe981f2b7516d02cc4a78eee7e9f44490f93aa08100c06c55fb7615bcc66c805";

    #[test(bob = @0x345)]
    #[expected_failure(abort_code = 65544)]
    public entry fun test_invalid_offer_signer_capability(bob: signer) acquires Account {
        let alice = create_account_from_ed25519_public_key(ALICE_PK);
        create_account(signer::address_of(&bob));

        // Maul the signature and make sure the call would fail
        let invalid_signature = ALICE_SIGNER_CAPABILITY_OFFER_SIGNATURE;

        let first_sig_byte = vector::borrow_mut(&mut invalid_signature, 0);
        *first_sig_byte = *first_sig_byte + 1;

        offer_signer_capability(&alice, invalid_signature, 0, ALICE_PK, signer::address_of(&bob));
    }

    #[test(bob = @0x345)]
    public entry fun test_valid_check_signer_capability_and_create_authorized_signer(bob: signer) acquires Account {
        let alice = create_account_from_ed25519_public_key(ALICE_PK);
        assert!(signer::address_of(&alice) == from_bcs::to_address(ALICE_ADDRESS), 0);

        create_account(signer::address_of(&bob));

        offer_signer_capability(&alice, ALICE_SIGNER_CAPABILITY_OFFER_SIGNATURE, 0, ALICE_PK, signer::address_of(&bob));

        let alice_account_resource = borrow_global_mut<Account>(signer::address_of(&alice));
        assert!(option::contains(&alice_account_resource.signer_capability_offer.for, &signer::address_of(&bob)), 0);

        let signer = create_authorized_signer(&bob, signer::address_of(&alice));
        assert!(signer::address_of(&signer) == signer::address_of(&alice), 0);
    }

    #[test(bob = @0x345, charlie = @0x567)]
    #[expected_failure(abort_code = 393230)]
    public entry fun test_invalid_check_signer_capability_and_create_authorized_signer(bob: signer, charlie: signer) acquires Account {
        let alice = create_account_from_ed25519_public_key(ALICE_PK);
        create_account(signer::address_of(&bob));

        offer_signer_capability(&alice, ALICE_SIGNER_CAPABILITY_OFFER_SIGNATURE, 0, ALICE_PK, signer::address_of(&bob));

        let alice_account_resource = borrow_global_mut<Account>(signer::address_of(&alice));
        assert!(option::contains(&alice_account_resource.signer_capability_offer.for, &signer::address_of(&bob)), 0);

        create_authorized_signer(&charlie, signer::address_of(&alice));
    }

    #[test(bob = @0x345)]
    public entry fun test_valid_revoke_signer_capability(bob: signer) acquires Account {
        let alice = create_account_from_ed25519_public_key(ALICE_PK);
        create_account(signer::address_of(&bob));

        offer_signer_capability(&alice, ALICE_SIGNER_CAPABILITY_OFFER_SIGNATURE, 0, ALICE_PK, signer::address_of(&bob));
        revoke_signer_capability(&alice, signer::address_of(&bob));
    }

    #[test(bob = @0x345, charlie = @0x567)]
    #[expected_failure(abort_code = 393230)]
    public entry fun test_invalid_revoke_signer_capability(bob: signer, charlie: signer) acquires Account {
        let alice = create_account_from_ed25519_public_key(ALICE_PK);
        create_account(signer::address_of(&bob));
        create_account(signer::address_of(&charlie));

        offer_signer_capability(&alice, ALICE_SIGNER_CAPABILITY_OFFER_SIGNATURE, 0, ALICE_PK, signer::address_of(&bob));
        revoke_signer_capability(&alice, signer::address_of(&charlie));
    }

    //
    // Tests for key rotation
    //

    // TODO: Add command used to generate these test cases
    #[test_only]
    const MULTI_ED25519_SRC_PK_BYTES: vector<u8> = x"2dfac52b88c70c98b2679959e5379af58e7907f23ca45e31873fee9b1adcbcc8179c572e0038769a35b3db83be05f288c6cc19393eb7455ea702a6ed87b01ed58b1550e3757ff1e927544f90bc2ea58cfcf9059956168c3012f71da09c242d6c4ff03de8a09ae3784774a6bead12f4ed9ad06d48555147a86c109016b4c21e5ab1eff5570df963c1d8b0ed6428602ce5ae06232e41af8eedb6a566b1f2f20df63206ece1de8530766c24e59ed6e4587e8d24829a1019860e0ec30bb0a9b7ea4f34d087247ab89f9538d6848f771468d5960c7c61bd72bc01cc1f1e2f43bb10a59c951b66f706a167ef516a7b3ccd742504032d0404257b7b7f6538763579353d45b982fa36d17b543b9136a0984e58b1abc019814d2d819e034fdb2cc5f727ad1d2cb4772af26385c745b55417efaacf745486ee5067982bea16c03620d215621038f3d8a266362f8ea913e7332e6738a09f44219908e2e4c76b791a24806692c385b4d31a47c49237eaa2abf93ce59a1b5ea20b0ae4b54911168aaecf61c4f74a18f60fa5364a2a3e04a508ed31d7426e2ffcfe54cfaa205c5e04d7ea6f174c6c090720e9fcb55f193dce4177e8d7b775a56667ae7c030e29cfbf710acb52c4de9920238fda835267d8166afab29594d74bed27d78d385be849a213ab4f1908c63317895e0da4d2963ca7ec0e06167f59aea9b6b5e007cd409a08f5329581579022f517920be5c16dfd3af7be3bfb48e07be9542237da8922bc8a38fec2d2f80b6b3ddbebfc43c98e841ed9e0e09588a1fc5d3faf815b02a8c19aebc76b175ea11c16eb7d492d3eaba92021858a8a22894adba3d069da5a7f61a3d00a72a5c402";
    #[test_only]
    const MULTI_ED25519_TO_PK_BYTES: vector<u8> = x"610352fe6f2e18dd80ecd4c5753f257cbdf773b814804ef923d566b20aea73e2259aad4da612d183eee6ad2b31f98e53e4372d6e7d7324e93dbeab8e9007fc49eaf8c3c85cc70d86a3f0f02b35a709f85a678918ac77344d4fb9ec25cf0fbc8fc65c581b15131d8e8d7c598f6c6f61b4b708a63fc885bb0662b54d7c24fab28e56992d9d837ecafa69a793dbd65311ea51939ca0e9186d172740cc3bbde80d1eb2c5687f2d083e15e298aff15d6fc79cf674200acaada18928454bace5231bcc0e7c2740e555ebda5ff8ee64d02fe803cb41b0080f7545f8a3f152fd0096494ac64e369366dccf268a8b9016fccae3e0c59e2b5a520aa987917d5c76ca6756d897c616f99590dfefe97bd0ddbb1506a7431f8ef54c0ef0e039e76fd740d0c6f490bf9e88c8ec4e6927dd17548a2f618499c5a08050129fce202b068621cf054123a0a4921d7aebb6a518f64654df247a24d86d040af0edf41619b0cdc5edf0d46ac3a3581fc381f1e69b06324a452022a6a4a3d3a225925b9f9ae1d8178b87fb347b551a1ce71ba91297862e3bdbccce3f8d3d6d1fa0706a6ee81d6148956202698eff98befebc472908eb293b2b6f5edb71da84adf5b08c84837942927ce1b4e30840d952427fd8c6bf9c4130e5de6886ee0982c9a1e17d68acd1ad4bc0a13fcdeebf65cb6257fd988c227e0324ccf505dee191f0422ffa256cb9d2a3b8b1eb0f";
    #[test_only]
    const SIG_CAP_ROTATE_KEY_MULTI_TO_MULTI: vector<u8> = x"79dc94f409a80bd9a1fe8c5e39931a7550dc9c9fe34b074f846545a862cf5ea9a0115014eaa84ce815f7531c8a95e9a409fab0952708739d0bbe1505406e8e0c645698ba17f527acf26de74c7981ff2cb8f344c9b614c7bb19e8777aa229be48142d25b3b0a603afdafeef37d7b58b7fcbf584207bef1c863788a9d94f173209c0000000";
    #[test_only]
    const SIG_CAP_UPDATE_TABLE_MULTI_TO_MULTI: vector<u8> = x"3e38df1bad449570ee8c6680bde7a59568797ec15e3f413b3c7d46903f07deea276b944e3cf37d27130e7de1fdbfb2dc240828224d83c4fb1480ba3573015404ce9ea4bfb05372da83dcd9926fb31a5dda89c61f6ae71b9f7f7cb96e2fd6a360aab999776e0f9a158755008676d3a9f64d343873722d285687280adaab3a5208e33e0ad98cf869a46fa6ad60ab1058098da1c45399047db492bc3638acd51bc1b37b8f4eecc969f48af2edd97e5ce46e7c9e430e6974ed630d7232e2ecddb104265379aac05c5397af7bf052359e7ba03744bbfe5ec6029a1992a115727deb3ad4b5b16b0182280b2709645ed21a208ac594dd2d49ee98b92f8ddff4b0e9760ad5461d05a20a8957cd7021369cc6de6553f1ab0ab272fa6da0ade46d5cb64e1015a16bc50f4ba25ac45fd9e5493b5b514c7abac4b5a19abeb8301c9ca0a548007ab9bd56c1a8284bd2e95652391cc86c760a717455d20339435008913b168b30af3958168787355c068ef48fd5e15b80b2ce17428297a85d927dbe1803e4990ab12e4bd64467c980bec84d6bfb05b3034f0171f292510f4ba49f285fe371b98985ce9cc4152d26d1e0be628e63f949c67d44fe9f667ef19a636106401b170e0463e1c54ec9eda985c59fcd76f1fc6733b389c2a98c112b10dcdecc93b4e5e65cec92ac8c0cd7cf1ee0ea2fdc3633a4c4b5e8035fc925854cdfa374940bc9270268eb9ef2bcbf7c0b4ed6818461468fd3914b256add96738ef932dc9487d5d59d5d70ec1a2227475446ba4cb28399f9875902492dab203e71cf47e1f605962608a02d032039ca11539fb9d883e26d0288f4e1a0b43fd33ddb5ee046c688a7854e88d57555a43d404b9d399832f964b8d029b7e2620fc4d9d1d96d6f40ee039f02e6390b0b18b08fb873886af680504ff4e07c548c1b7202566ec6412870446ce398929a1a64a658ff0dfc0225da3bbdb75f82a1a0288ff6cdde687ef3b66af20e3369d398460cb5b91c62388c713a31be1b4172a47506fd71629df175946dc5deaeb9bafa736463c82aa761effe54b4810f0db65b57e47ee7ca341e309bbcb305c53f66b1be09a1d7d6213eb7d065bb5a0995168de71905ab20c2c71ae540117a00a632ed92185a615988b766812246019c550a6ce43819faa01a9aba5274af06d4ed86c609c3cf21ac302558c0e7b480ee784c7a61d5c374a733d08956191b0d6df304a2e12831258e1b933d34327eb307a23182f21e16593231772edc9996053240d4a4a21763003b4fb56d5a5a2a28a29399c49b4f0baec8677447ca3059bdd62c08cb2efb76e484f831d665ab30a5acd3e55cbff220451b504e8c8d495a0dfffe0000";
    #[test_only]
    const ED25519_TO_PK_BYTES: vector<u8> = x"610352fe6f2e18dd80ecd4c5753f257cbdf773b814804ef923d566b20aea73e2";
    #[test_only]
    const SIG_CAP_ROTATE_KEY_MULTI_TO_SINGLE: vector<u8> = x"9bd379b45d6e46da926c3dface9f673877cdd527871dc71e7dd563a6cba9b59eb5df6bb14c7491bb71e31bf3ba04fb38cf92ec5219ea8010d525d567fa65e601d65362fe7a3ac2e63d60319538dbfc25e4f958940cd1f03054809ffbcb6de6ea6fb28b05aefe6fd4ff62d184dc204f1dcb9e3627b5928dca5d9a62114bb2a80cc0000000";
    #[test_only]
    const SIG_CAP_UPDATE_TABLE_MULTI_TO_SINGLE: vector<u8> = x"ec0c10a20af0be64c01b3643373b9622ef9e572e1e6e3f1fa2d03ca23edb852b71c9dea225fc85442170680653709a31c7d21c2c0777ac31fd91fd0821c74e04";
    #[test_only]
    const SIG_CAP_ROTATE_KEY_MULTI_TO_MULTI_SIMPLIFY: vector<u8> = x"919f69fffacd3d41e2cd068e86dd1df167db9c6340a16ec86113d3e2228acc914ba58e73c1754c309a0d5926e2ebc9b5fa9227a480ffddb90377f34d8d75b10e71231129bf22c045fbd95df90e0c63f1439cff354be21599721b9c19d54f5204af07b1589486550a337a34e230ea435b9a41d31552a9805ce644156e4e602405c0000000";
    #[test_only]
    const SIG_CAP_ROTATE_KEY_MULTI_TO_SINGLE_SIMPLIFY: vector<u8> = x"1a33145cbbfb2e45820c6273b240b136f3caddfbb4eafda786dc861bfe848e761190c296aa0893da7fdd225839b4acf2d8fbf446e9d20087d11bc3ddb3c7050ea8d72d267275c5c220116b77dc11f252fd0f2985e8a37c713464219071baa6053f67130191659d579bd7cf0f940ee67a9b491c090c860a302b4710e936cc0c03c0000000";


    #[test(account = @aptos_framework)]
    public entry fun test_valid_rotate_authentication_key_multi_ed25519_to_multi_ed25519(account: signer) acquires Account, OriginatingAddress {
        initialize(&account);

        let curr_pk = multi_ed25519::new_unvalidated_public_key_from_bytes(MULTI_ED25519_SRC_PK_BYTES);
        let curr_auth_key = multi_ed25519::unvalidated_public_key_to_authentication_key(&curr_pk);
        let alice_address = from_bcs::to_address(curr_auth_key);
        let alice = create_account_unchecked(alice_address);

        rotate_authentication_key(&alice, MULTI_ED25519_SCHEME, MULTI_ED25519_SRC_PK_BYTES, MULTI_ED25519_SCHEME, MULTI_ED25519_TO_PK_BYTES, SIG_CAP_ROTATE_KEY_MULTI_TO_MULTI, SIG_CAP_UPDATE_TABLE_MULTI_TO_MULTI);
        let address_map = &mut borrow_global_mut<OriginatingAddress>(@aptos_framework).address_map;
        let new_pk = multi_ed25519::new_unvalidated_public_key_from_bytes(MULTI_ED25519_TO_PK_BYTES);
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

        let curr_pk = multi_ed25519::new_unvalidated_public_key_from_bytes(MULTI_ED25519_SRC_PK_BYTES);
        let curr_auth_key = multi_ed25519::unvalidated_public_key_to_authentication_key(&curr_pk);
        let alice_address = from_bcs::to_address(curr_auth_key);
        let alice = create_account_unchecked(alice_address);

        rotate_authentication_key(&alice, MULTI_ED25519_SCHEME, MULTI_ED25519_SRC_PK_BYTES, ED25519_SCHEME, ED25519_TO_PK_BYTES, SIG_CAP_ROTATE_KEY_MULTI_TO_SINGLE, SIG_CAP_UPDATE_TABLE_MULTI_TO_SINGLE);
        let address_map = &mut borrow_global_mut<OriginatingAddress>(@aptos_framework).address_map;
        let new_pk = ed25519::new_unvalidated_public_key_from_bytes(ED25519_TO_PK_BYTES);
        let new_auth_key = ed25519::unvalidated_public_key_to_authentication_key(&new_pk);
        let new_address = from_bcs::to_address(new_auth_key);
        let expected_originating_address = table::borrow(address_map, new_address);
        assert!(*expected_originating_address == alice_address, 0);

        let account_resource = borrow_global_mut<Account>(alice_address);
        assert!(account_resource.authentication_key == new_auth_key, 0);
    }

    #[test(account = @aptos_framework)]
    public entry fun test_valid_rotate_authentication_key_multi_ed25519_to_multi_ed25519_simplify(account: signer) acquires Account {
        initialize(&account);

        let curr_pk = multi_ed25519::new_unvalidated_public_key_from_bytes(MULTI_ED25519_SRC_PK_BYTES);
        let curr_auth_key = multi_ed25519::unvalidated_public_key_to_authentication_key(&curr_pk);
        let alice_address = from_bcs::to_address(curr_auth_key);
        create_account_unchecked(alice_address);

        rotate_authentication_key_simplify(alice_address, MULTI_ED25519_SCHEME, MULTI_ED25519_SRC_PK_BYTES, MULTI_ED25519_SCHEME, MULTI_ED25519_TO_PK_BYTES, SIG_CAP_ROTATE_KEY_MULTI_TO_MULTI_SIMPLIFY);

        let new_pk = multi_ed25519::new_unvalidated_public_key_from_bytes(MULTI_ED25519_TO_PK_BYTES);
        let new_auth_key = multi_ed25519::unvalidated_public_key_to_authentication_key(&new_pk);

        let account_resource = borrow_global_mut<Account>(alice_address);
        assert!(account_resource.authentication_key == new_auth_key, 0);
    }


    #[test(account = @aptos_framework)]
    public entry fun test_valid_rotate_authentication_key_multi_ed25519_to_ed25519_simplify(account: signer) acquires Account {
        initialize(&account);

        let curr_pk = multi_ed25519::new_unvalidated_public_key_from_bytes(MULTI_ED25519_SRC_PK_BYTES);
        let curr_auth_key = multi_ed25519::unvalidated_public_key_to_authentication_key(&curr_pk);
        let alice_address = from_bcs::to_address(curr_auth_key);
        create_account_unchecked(alice_address);
        rotate_authentication_key_simplify(alice_address, MULTI_ED25519_SCHEME, MULTI_ED25519_SRC_PK_BYTES, ED25519_SCHEME, ED25519_TO_PK_BYTES, SIG_CAP_ROTATE_KEY_MULTI_TO_SINGLE_SIMPLIFY);

        let new_pk = ed25519::new_unvalidated_public_key_from_bytes(ED25519_TO_PK_BYTES);
        let new_auth_key = ed25519::unvalidated_public_key_to_authentication_key(&new_pk);

        let account_resource = borrow_global_mut<Account>(alice_address);
        assert!(account_resource.authentication_key == new_auth_key, 0);
    }
}
