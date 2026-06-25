spec aptos_framework::account {
    /// <high-level-req>
    /// No.: 1
    /// Requirement: The initialization of the account module should result in the proper system initialization with valid
    /// and consistent resources.
    /// Criticality: High
    /// Implementation: Initialization of the account module creates a valid address_map table and moves the resources
    /// to the OriginatingAddress under the aptos_framework account.
    /// Enforcement: Audited that the address_map table is created and populated correctly with the expected initial
    /// values.
    ///
    /// No.: 2
    /// Requirement: After successfully creating an account, the account resources should initialize with the default data,
    /// ensuring the proper initialization of the account state.
    /// Criticality: High
    /// Implementation: Creating an account via the create_account function validates the state and moves a new account
    /// resource under new_address.
    /// Enforcement: Formally verified via [high-level-req-2](create_account).
    ///
    /// No.: 3
    /// Requirement: Checking the existence of an account under a given address never results in an abort.
    /// Criticality: Low
    /// Implementation: The exists_at function returns a boolean value indicating the existence of an account under the
    /// given address.
    /// Enforcement: Formally verified by the [high-level-req-3](aborts_if) condition.
    ///
    /// No.: 4
    /// Requirement: The account module maintains bounded sequence numbers for all accounts, guaranteeing they remain
    /// within the specified limit.
    /// Criticality: Medium
    /// Implementation: The sequence number of an account may only increase up to MAX_U64 in a succeeding manner.
    /// Enforcement: Formally verified via [high-level-req-4](increment_sequence_number) that it remains within the defined boundary of
    /// MAX_U64.
    ///
    /// No.: 5
    /// Requirement: Only the ed25519 and multied25519 signature schemes are permissible.
    /// Criticality: Low
    /// Implementation: Exclusively perform key rotation using either the ed25519 or multied25519 signature schemes.
    /// Currently restricts the offering of rotation/signing capabilities to the ed25519 or multied25519 schemes.
    /// Enforcement: Formally Verified: [high-level-req-5.1](rotate_authentication_key),
    /// [high-level-req-5.2](offer_rotation_capability), and [high-level-req-5.3](offer_signer_capability).
    /// Verified that it aborts if the account_scheme is not ED25519_SCHEME and not MULTI_ED25519_SCHEME. Audited
    /// that the scheme enums correspond correctly to signature logic.
    ///
    /// No.: 6
    /// Requirement: Exclusively permit the rotation of the authentication key of an account for the account owner or any
    /// user who possesses rotation capabilities associated with that account.
    /// Criticality: Critical
    /// Implementation: In the rotate_authentication_key function, the authentication key derived from the
    /// from_public_key_bytes should match the signer's current authentication key. Only the delegate_signer granted the
    /// rotation capabilities may invoke the rotate_authentication_key_with_rotation_capability function.
    /// Enforcement: Formally Verified via [high-level-req-6.1](rotate_authentication_key) and
    /// [high-level-req-6.2](rotate_authentication_key_with_rotation_capability).
    ///
    /// No.: 7
    /// Requirement: Only the owner of an account may offer or revoke the following capabilities: (1)
    /// offer_rotation_capability, (2) offer_signer_capability, (3) revoke_rotation_capability, and (4)
    /// revoke_signer_capability.
    /// Criticality: Critical
    /// Implementation: An account resource may only be modified by the owner of the account utilizing:
    /// rotation_capability_offer, signer_capability_offer.
    /// Enforcement: Formally verified via [high-level-req-7.1](offer_rotation_capability),
    /// [high-level-req-7.2](offer_signer_capability), and [high-level-req-7.3](revoke_rotation_capability).
    /// and [high-level-req-7.4](revoke_signer_capability).
    ///
    /// No.: 8
    /// Requirement: The capability to create a signer for the account is exclusively reserved for either the account owner
    /// or the account that has been granted the signing capabilities.
    /// Criticality: Critical
    /// Implementation: Signer creation for the account may only be successfully executed by explicitly granting the
    /// signing capabilities with the create_authorized_signer function.
    /// Enforcement: Formally verified via [high-level-req-8](create_authorized_signer).
    ///
    /// No.: 9
    /// Requirement: Rotating the authentication key requires two valid signatures. With the private key of the current
    /// authentication key. With the private key of the new authentication key.
    /// Criticality: Critical
    /// Implementation: The rotate_authentication_key verifies two signatures (current and new) before rotating to the
    /// new key. The first signature ensures the user has the intended capability, and the second signature ensures that
    /// the user owns the new key.
    /// Enforcement: Formally verified via [high-level-req-9.1](rotate_authentication_key) and
    /// [high-level-req-9.2](rotate_authentication_key_with_rotation_capability).
    ///
    /// No.: 10
    /// Requirement: The rotation of the authentication key updates the account's authentication key with the newly supplied
    /// one.
    /// Criticality: High
    /// Implementation: The auth_key may only update to the provided new_auth_key after verifying the signature.
    /// Enforcement: Formally Verified in [high-level-req-10](rotate_authentication_key_internal) that the
    /// authentication key of an account is modified to the provided authentication key if the signature verification
    /// was successful.
    ///
    /// No.: 11
    /// Requirement: The creation number is monotonically increasing.
    /// Criticality: Low
    /// Implementation: The guid_creation_num in the Account structure is monotonically increasing.
    /// Enforcement: Formally Verified via [high-level-req-11](guid_creation_num).
    ///
    /// No.: 12
    /// Requirement: The Account resource is persistent.
    /// Criticality: Low
    /// Implementation: The Account structure assigned to the address should be persistent.
    /// Enforcement: Audited that the Account structure is persistent.
    /// </high-level-req>
    ///

    spec module {
        pragma verify = true;
    }

    /// Only the address `@aptos_framework` can call.
    /// OriginatingAddress does not exist under `@aptos_framework` before the call.
    spec initialize(aptos_framework: &signer) {
        let aptos_addr = signer::address_of(aptos_framework);
        aborts_if !system_addresses::is_aptos_framework_address(aptos_addr);
        aborts_if exists<OriginatingAddress>(aptos_addr);
        ensures exists<OriginatingAddress>(aptos_addr);
    }

    /// Ensure that the account exists at the end of the call.
    spec create_account_if_does_not_exist(account_address: address) {
        let authentication_key = bcs::to_bytes(account_address);
        modifies global<Account>(account_address);
        aborts_if !exists<Account>(account_address) && (
            account_address == @vm_reserved
            || account_address == @aptos_framework
            || account_address == @aptos_token
            || !(len(authentication_key) == 32)
        );
        ensures exists<Account>(account_address);
    }

    spec ensure_resource_exists(addr: address) {
        pragma opaque;
        include EnsureResourceExistsAbortsIf;
        modifies global<Account>(addr);
        ensures exists<Account>(addr);
        ensures old(exists<Account>(addr)) ==>
            global<Account>(addr) == old(global<Account>(addr));
        ensures !old(exists<Account>(addr)) ==>
            global<Account>(addr).authentication_key == bcs::to_bytes(addr)
                && global<Account>(addr).sequence_number == 0
                && global<Account>(addr).guid_creation_num == 2;
    }




    /// Check if the bytes of the new address is 32.
    /// The Account does not exist under the new address before creating the account.
    /// Limit the new account address is not @vm_reserved / @aptos_framework / @aptos_toke.
    spec create_account(new_address: address): signer {
        use std::features::{Self, DEFAULT_ACCOUNT_RESOURCE};
        let feature_on = features::spec_is_enabled(DEFAULT_ACCOUNT_RESOURCE);
        aborts_if exists<Account>(new_address);
        aborts_if new_address == @vm_reserved || new_address == @aptos_framework || new_address == @aptos_token;
        aborts_if !feature_on && len(bcs::to_bytes(new_address)) != 32;
        ensures signer::address_of(result) == new_address;
        /// [high-level-req-2]
        ensures !feature_on ==> exists<Account>(new_address);
        ensures !signer::spec_is_permissioned_signer_impl(result);
    }

    /// Check if the bytes of the new address is 32.
    /// The Account does not exist under the new address before creating the account.
    spec create_account_unchecked(new_address: address): signer {
        pragma opaque;
        include CreateAccountAbortsIf {addr: new_address};
        modifies global<Account>(new_address);
        ensures signer::address_of(result) == new_address;
        ensures exists<Account>(new_address);
        ensures global<Account>(new_address).guid_creation_num == 2;
        ensures !signer::spec_is_permissioned_signer_impl(result);
        ensures global<Account>(new_address).sequence_number == 0;
        ensures global<Account>(new_address).authentication_key == bcs::to_bytes(new_address);
    }

    spec exists_at {
        pragma opaque;
        /// [high-level-req-3]
        aborts_if false;
        ensures result == spec_exists_at(addr);
    }

    spec fun spec_exists_at(addr: address): bool {
        use std::features;
        use std::features::DEFAULT_ACCOUNT_RESOURCE;
        features::spec_is_enabled(DEFAULT_ACCOUNT_RESOURCE) || exists<Account>(addr)
    }

    spec schema CreateAccountAbortsIf {
        addr: address;
        let authentication_key = bcs::to_bytes(addr);
        aborts_if len(authentication_key) != 32;
        aborts_if exists<Account>(addr);
        ensures len(authentication_key) == 32;
    }

    spec get_guid_next_creation_num(addr: address): u64 {
        use std::features::{Self, DEFAULT_ACCOUNT_RESOURCE};
        let feature_on = features::spec_is_enabled(DEFAULT_ACCOUNT_RESOURCE);
        aborts_if !exists<Account>(addr) && !feature_on;
        ensures result == (
            if (exists<Account>(addr)) global<Account>(addr).guid_creation_num else 0
        );
    }

    spec get_sequence_number(addr: address): u64 {
        use std::features::{Self, DEFAULT_ACCOUNT_RESOURCE};
        let feature_on = features::spec_is_enabled(DEFAULT_ACCOUNT_RESOURCE);
        aborts_if !exists<Account>(addr) && !feature_on;
        ensures result == (
            if (exists<Account>(addr)) global<Account>(addr).sequence_number else 0
        );
    }

    spec increment_sequence_number(addr: address) {
        include EnsureResourceExistsAbortsIf;
        let sequence_number_pre = if (exists<Account>(addr)) global<Account>(addr).sequence_number else 0;
        /// [high-level-req-4]
        aborts_if sequence_number_pre == MAX_U64;
        modifies global<Account>(addr);
        ensures global<Account>(addr).sequence_number == sequence_number_pre + 1;
    }

    spec get_authentication_key(addr: address): vector<u8> {
        use std::features::{Self, DEFAULT_ACCOUNT_RESOURCE};
        pragma opaque;
        let feature_on = features::spec_is_enabled(DEFAULT_ACCOUNT_RESOURCE);
        aborts_if !exists<Account>(addr) && !feature_on;
        ensures result == spec_get_authentication_key(addr);
    }

    spec fun spec_get_authentication_key(addr: address): vector<u8> {
        if (exists<Account>(addr)) {
            global<Account>(addr).authentication_key
        } else {
            bcs::to_bytes(addr)
        }
    }

    /// The Account existed under the signer before the call.
    /// The length of new_auth_key is 32.
    spec rotate_authentication_key_internal(account: &signer, new_auth_key: vector<u8>) {
        let addr = signer::address_of(account);
        include AccountPermissionAbortsIf<AccountPermission> { perm: AccountPermission::KeyRotation {} };
        include EnsureResourceExistsAbortsIf;
        aborts_if vector::length(new_auth_key) != 32;
        modifies global<Account>(addr);
        /// [high-level-req-10]
        ensures global<Account>(addr).authentication_key == new_auth_key;
    }

    spec rotate_authentication_key_call(account: &signer, new_auth_key: vector<u8>) {
        let addr = signer::address_of(account);
        include AccountPermissionAbortsIf<AccountPermission> { perm: AccountPermission::KeyRotation {} };
        include EnsureResourceExistsAbortsIf;
        aborts_if vector::length(new_auth_key) != 32;
        modifies global<Account>(addr);
        /// [high-level-req-10]
        ensures global<Account>(addr).authentication_key == new_auth_key;
    }

    spec rotate_authentication_key_from_public_key(account: &signer, scheme: u8, new_public_key_bytes: vector<u8>) {
        pragma aborts_if_is_partial;
        include AccountPermissionAbortsIf<AccountPermission> { perm: AccountPermission::KeyRotation {} };
        let addr = signer::address_of(account);
        aborts_if !exists<Account>(addr);
        aborts_if scheme != ED25519_SCHEME && scheme != MULTI_ED25519_SCHEME
            && scheme != SINGLE_KEY_SCHEME && scheme != MULTI_KEY_SCHEME;
        include scheme == ED25519_SCHEME ==>
            ed25519::NewUnvalidatedPublicKeyFromBytesAbortsIf { bytes: new_public_key_bytes };
        include scheme == MULTI_ED25519_SCHEME ==>
            multi_ed25519::NewUnvalidatedPublicKeyFromBytesAbortsIf { bytes: new_public_key_bytes };
        modifies global<Account>(addr);
    }

    spec fun spec_assert_valid_rotation_proof_signature_and_get_auth_key(scheme: u8, public_key_bytes: vector<u8>, signature: vector<u8>, challenge: RotationProofChallenge): vector<u8>;

    spec assert_valid_rotation_proof_signature_and_get_auth_key(scheme: u8, public_key_bytes: vector<u8>, signature: vector<u8>, challenge: &RotationProofChallenge): vector<u8> {
        pragma verify = false;
        pragma opaque;
        include AssertValidRotationProofSignatureAndGetAuthKeyAbortsIf;
        ensures [abstract] result == spec_assert_valid_rotation_proof_signature_and_get_auth_key(scheme, public_key_bytes, signature, challenge);
    }
    spec schema AssertValidRotationProofSignatureAndGetAuthKeyAbortsIf {
        scheme: u8;
        public_key_bytes: vector<u8>;
        signature: vector<u8>;
        challenge: RotationProofChallenge;

        include scheme == ED25519_SCHEME ==> ed25519::NewUnvalidatedPublicKeyFromBytesAbortsIf { bytes: public_key_bytes };
        include scheme == ED25519_SCHEME ==> ed25519::NewSignatureFromBytesAbortsIf { bytes: signature };
        aborts_if scheme == ED25519_SCHEME && !ed25519::spec_signature_verify_strict_t(
            ed25519::Signature { bytes: signature },
            ed25519::UnvalidatedPublicKey { bytes: public_key_bytes },
            challenge
        );

        include scheme == MULTI_ED25519_SCHEME ==> multi_ed25519::NewUnvalidatedPublicKeyFromBytesAbortsIf { bytes: public_key_bytes };
        include scheme == MULTI_ED25519_SCHEME ==> multi_ed25519::NewSignatureFromBytesAbortsIf { bytes: signature };
        aborts_if scheme == MULTI_ED25519_SCHEME && !multi_ed25519::spec_signature_verify_strict_t(
            multi_ed25519::Signature { bytes: signature },
            multi_ed25519::UnvalidatedPublicKey { bytes: public_key_bytes },
            challenge
        );
        aborts_if scheme != ED25519_SCHEME && scheme != MULTI_ED25519_SCHEME;
    }

    /// The Account existed under the signer
    /// The authentication scheme is ED25519_SCHEME and MULTI_ED25519_SCHEME
    spec rotate_authentication_key(
        account: &signer,
        from_scheme: u8,
        from_public_key_bytes: vector<u8>,
        to_scheme: u8,
        to_public_key_bytes: vector<u8>,
        cap_rotate_key: vector<u8>,
        cap_update_table: vector<u8>,
    ) {
        pragma aborts_if_is_partial;
        include AccountPermissionAbortsIf<AccountPermission> { perm: AccountPermission::KeyRotation {} };
        let addr = signer::address_of(account);
        include EnsureResourceExistsAbortsIf { addr };
        /// [high-level-req-5.1]
        aborts_if from_scheme != ED25519_SCHEME && from_scheme != MULTI_ED25519_SCHEME;
        aborts_if to_scheme != ED25519_SCHEME && to_scheme != MULTI_ED25519_SCHEME;
        let current_auth_key_bytes = spec_get_authentication_key(addr);
        let current_seq = if (exists<Account>(addr)) global<Account>(addr).sequence_number else 0;

        include from_scheme == ED25519_SCHEME ==>
            ed25519::NewUnvalidatedPublicKeyFromBytesAbortsIf { bytes: from_public_key_bytes };
        include from_scheme == MULTI_ED25519_SCHEME ==>
            multi_ed25519::NewUnvalidatedPublicKeyFromBytesAbortsIf { bytes: from_public_key_bytes };
        /// [high-level-req-6.1]
        aborts_if from_scheme == ED25519_SCHEME &&
            current_auth_key_bytes !=
                ed25519::spec_public_key_bytes_to_authentication_key(from_public_key_bytes);
        aborts_if from_scheme == MULTI_ED25519_SCHEME &&
            current_auth_key_bytes !=
                multi_ed25519::spec_public_key_bytes_to_authentication_key(from_public_key_bytes);

        aborts_if !from_bcs::deserializable<address>(current_auth_key_bytes);
        let curr_auth_key = from_bcs::deserialize<address>(current_auth_key_bytes);
        let challenge = RotationProofChallenge {
            sequence_number: current_seq,
            originator: addr,
            current_auth_key: curr_auth_key,
            new_public_key: to_public_key_bytes,
        };

        /// [high-level-req-9.1]
        include AssertValidRotationProofSignatureAndGetAuthKeyAbortsIf {
            scheme: from_scheme,
            public_key_bytes: from_public_key_bytes,
            signature: cap_rotate_key,
            challenge,
        };
        include AssertValidRotationProofSignatureAndGetAuthKeyAbortsIf {
            scheme: to_scheme,
            public_key_bytes: to_public_key_bytes,
            signature: cap_update_table,
            challenge,
        };

        let new_auth_key_vector = spec_assert_valid_rotation_proof_signature_and_get_auth_key(
            to_scheme, to_public_key_bytes, cap_update_table, challenge);

        include UpdateAuthKeyAndOriginatingAddressTableAbortsIf {
            originating_addr: addr,
            current_authentication_key_bytes: current_auth_key_bytes,
            new_auth_key_vector,
        };

        /// [high-level-req-10]
        modifies global<Account>(addr);
        modifies global<OriginatingAddress>(@aptos_framework);
        let post auth_key = global<Account>(addr).authentication_key;
        ensures auth_key == new_auth_key_vector;
    }

    spec rotate_authentication_key_with_rotation_capability(
        delegate_signer: &signer,
        rotation_cap_offerer_address: address,
        new_scheme: u8,
        new_public_key_bytes: vector<u8>,
        cap_update_table: vector<u8>
    ) {
        include AccountPermissionAbortsIf<AccountPermission> { account: delegate_signer, perm: AccountPermission::KeyRotation {} };
        aborts_if !exists<Account>(rotation_cap_offerer_address);
        let delegate_address = signer::address_of(delegate_signer);
        let offerer_account_resource = global<Account>(rotation_cap_offerer_address);
        aborts_if !from_bcs::deserializable<address>(offerer_account_resource.authentication_key);
        let curr_auth_key = from_bcs::deserialize<address>(offerer_account_resource.authentication_key);
        let delegate_seq = if (exists<Account>(delegate_address)) global<Account>(delegate_address).sequence_number else 0;
        let challenge = RotationProofChallenge {
            sequence_number: delegate_seq,
            originator: rotation_cap_offerer_address,
            current_auth_key: curr_auth_key,
            new_public_key: new_public_key_bytes,
        };
        aborts_if !exists<Account>(delegate_address)
            && !std::features::spec_is_enabled(std::features::DEFAULT_ACCOUNT_RESOURCE);
        /// [high-level-req-6.2]
        aborts_if !option::spec_contains(offerer_account_resource.rotation_capability_offer.for, delegate_address);
        /// [high-level-req-9.2]
        include AssertValidRotationProofSignatureAndGetAuthKeyAbortsIf {
            scheme: new_scheme,
            public_key_bytes: new_public_key_bytes,
            signature: cap_update_table,
            challenge,
        };

        let new_auth_key_vector = spec_assert_valid_rotation_proof_signature_and_get_auth_key(new_scheme, new_public_key_bytes, cap_update_table, challenge);
        let address_map = global<OriginatingAddress>(@aptos_framework).address_map;

        // Verify all properties in update_auth_key_and_originating_address_table
        aborts_if !exists<OriginatingAddress>(@aptos_framework);
        aborts_if !from_bcs::deserializable<address>(offerer_account_resource.authentication_key);
        aborts_if table::spec_contains(address_map, curr_auth_key) &&
            table::spec_get(address_map, curr_auth_key) != rotation_cap_offerer_address;

        aborts_if !from_bcs::deserializable<address>(new_auth_key_vector);
        let new_auth_key = from_bcs::deserialize<address>(new_auth_key_vector);

        aborts_if curr_auth_key != new_auth_key && table::spec_contains(address_map, new_auth_key);
        include UpdateAuthKeyAndOriginatingAddressTableAbortsIf {
            originating_addr: rotation_cap_offerer_address,
            current_authentication_key_bytes: offerer_account_resource.authentication_key,
        };

        let post auth_key = global<Account>(rotation_cap_offerer_address).authentication_key;
        ensures auth_key == new_auth_key_vector;
    }

    spec offer_rotation_capability(
        account: &signer,
        rotation_capability_sig_bytes: vector<u8>,
        account_scheme: u8,
        account_public_key_bytes: vector<u8>,
        recipient_address: address,
    ) {
        pragma aborts_if_is_partial;
        include AccountPermissionAbortsIf<AccountPermission> { perm: AccountPermission::KeyRotation {} };
        let source_address = signer::address_of(account);
        include EnsureResourceExistsAbortsIf { addr: source_address };
        let current_auth_key_bytes = spec_get_authentication_key(source_address);
        aborts_if !exists<chain_id::ChainId>(@aptos_framework);
        aborts_if !spec_exists_at(recipient_address);
        /// [high-level-req-5.2]
        aborts_if account_scheme != ED25519_SCHEME && account_scheme != MULTI_ED25519_SCHEME;

        include account_scheme == ED25519_SCHEME ==>
            ed25519::NewUnvalidatedPublicKeyFromBytesAbortsIf { bytes: account_public_key_bytes };
        include account_scheme == MULTI_ED25519_SCHEME ==>
            multi_ed25519::NewUnvalidatedPublicKeyFromBytesAbortsIf { bytes: account_public_key_bytes };
        include account_scheme == ED25519_SCHEME ==>
            ed25519::NewSignatureFromBytesAbortsIf { bytes: rotation_capability_sig_bytes };
        include account_scheme == MULTI_ED25519_SCHEME ==>
            multi_ed25519::NewSignatureFromBytesAbortsIf { bytes: rotation_capability_sig_bytes };
        aborts_if account_scheme == ED25519_SCHEME &&
            current_auth_key_bytes !=
                ed25519::spec_public_key_bytes_to_authentication_key(account_public_key_bytes);
        aborts_if account_scheme == MULTI_ED25519_SCHEME &&
            current_auth_key_bytes !=
                multi_ed25519::spec_public_key_bytes_to_authentication_key(account_public_key_bytes);

        /// [high-level-req-7.1]
        modifies global<Account>(source_address);
        let post offer_for = global<Account>(source_address).rotation_capability_offer.for;
        ensures option::borrow(offer_for) == recipient_address;
    }

    /// The Account existed under the signer.
    /// The authentication scheme is ED25519_SCHEME and MULTI_ED25519_SCHEME.
    spec offer_signer_capability(
        account: &signer,
        signer_capability_sig_bytes: vector<u8>,
        account_scheme: u8,
        account_public_key_bytes: vector<u8>,
        recipient_address: address
    ) {
        pragma aborts_if_is_partial;
        include AccountPermissionAbortsIf<AccountPermission> { perm: AccountPermission::Offering {} };
        let source_address = signer::address_of(account);
        include EnsureResourceExistsAbortsIf { addr: source_address };
        let current_auth_key_bytes = spec_get_authentication_key(source_address);
        aborts_if !spec_exists_at(recipient_address);
        /// [high-level-req-5.3]
        aborts_if account_scheme != ED25519_SCHEME && account_scheme != MULTI_ED25519_SCHEME;

        include account_scheme == ED25519_SCHEME ==>
            ed25519::NewUnvalidatedPublicKeyFromBytesAbortsIf { bytes: account_public_key_bytes };
        include account_scheme == MULTI_ED25519_SCHEME ==>
            multi_ed25519::NewUnvalidatedPublicKeyFromBytesAbortsIf { bytes: account_public_key_bytes };
        include account_scheme == ED25519_SCHEME ==>
            ed25519::NewSignatureFromBytesAbortsIf { bytes: signer_capability_sig_bytes };
        include account_scheme == MULTI_ED25519_SCHEME ==>
            multi_ed25519::NewSignatureFromBytesAbortsIf { bytes: signer_capability_sig_bytes };
        aborts_if account_scheme == ED25519_SCHEME &&
            current_auth_key_bytes !=
                ed25519::spec_public_key_bytes_to_authentication_key(account_public_key_bytes);
        aborts_if account_scheme == MULTI_ED25519_SCHEME &&
            current_auth_key_bytes !=
                multi_ed25519::spec_public_key_bytes_to_authentication_key(account_public_key_bytes);

        /// [high-level-req-7.2]
        modifies global<Account>(source_address);
        let post offer_for = global<Account>(source_address).signer_capability_offer.for;
        ensures option::borrow(offer_for) == recipient_address;
    }

    spec is_signer_capability_offered(account_addr: address): bool {
        use std::features::{Self, DEFAULT_ACCOUNT_RESOURCE};
        let feature_on = features::spec_is_enabled(DEFAULT_ACCOUNT_RESOURCE);
        aborts_if !exists<Account>(account_addr) && !feature_on;
    }

    spec get_signer_capability_offer_for(account_addr: address): address {
        aborts_if !exists<Account>(account_addr);
        let account_resource = global<Account>(account_addr);
        aborts_if option::is_none(account_resource.signer_capability_offer.for);
    }

    spec is_rotation_capability_offered(account_addr: address): bool {
        use std::features::{Self, DEFAULT_ACCOUNT_RESOURCE};
        let feature_on = features::spec_is_enabled(DEFAULT_ACCOUNT_RESOURCE);
        aborts_if !exists<Account>(account_addr) && !feature_on;
    }

    spec get_rotation_capability_offer_for(account_addr: address): address {
        aborts_if !exists<Account>(account_addr);
        let account_resource = global<Account>(account_addr);
        aborts_if option::is_none(account_resource.rotation_capability_offer.for);
    }

    /// The Account existed under the signer.
    /// The value of signer_capability_offer.for of Account resource under the signer is to_be_revoked_address.
    spec revoke_signer_capability(account: &signer, to_be_revoked_address: address) {
        include AccountPermissionAbortsIf<AccountPermission> { perm: AccountPermission::Offering {} };
        aborts_if !spec_exists_at(to_be_revoked_address);
        let addr = signer::address_of(account);
        let account_resource = global<Account>(addr);
        aborts_if !exists<Account>(addr);
        aborts_if !option::spec_contains(account_resource.signer_capability_offer.for,to_be_revoked_address);
        modifies global<Account>(addr);
    }

    spec revoke_any_signer_capability(account: &signer) {
        include AccountPermissionAbortsIf<AccountPermission> { perm: AccountPermission::Offering {} };
        modifies global<Account>(signer::address_of(account));
        /// [high-level-req-7.4]
        aborts_if !exists<Account>(signer::address_of(account));
        let account_resource = global<Account>(signer::address_of(account));
        aborts_if !option::is_some(account_resource.signer_capability_offer.for);
    }

    spec revoke_rotation_capability(account: &signer, to_be_revoked_address: address) {
        include AccountPermissionAbortsIf<AccountPermission> { perm: AccountPermission::KeyRotation {} };
        aborts_if !spec_exists_at(to_be_revoked_address);
        let addr = signer::address_of(account);
        let account_resource = global<Account>(addr);
        aborts_if !exists<Account>(addr);
        aborts_if !option::spec_contains(account_resource.rotation_capability_offer.for,to_be_revoked_address);
        modifies global<Account>(addr);
        let post offer_for = global<Account>(addr).rotation_capability_offer.for;
        ensures !option::is_some(offer_for);
    }

    spec revoke_any_rotation_capability(account: &signer) {
        include AccountPermissionAbortsIf<AccountPermission> { perm: AccountPermission::KeyRotation {} };
        let addr = signer::address_of(account);
        modifies global<Account>(addr);
        aborts_if !exists<Account>(addr);
        let account_resource = global<Account>(addr);
        /// [high-level-req-7.3]
        aborts_if !option::is_some(account_resource.rotation_capability_offer.for);
        let post offer_for = global<Account>(addr).rotation_capability_offer.for;
        ensures !option::is_some(offer_for);
    }

    /// The Account existed under the signer.
    /// The value of signer_capability_offer.for of Account resource under the signer is offerer_address.
    spec create_authorized_signer(account: &signer, offerer_address: address): signer {
        include AccountPermissionAbortsIf<AccountPermission> { perm: AccountPermission::Offering {} };
        /// [high-level-req-8]
        include AccountContainsAddr{
            account,
            address: offerer_address,
        };
        modifies global<Account>(offerer_address);
        ensures exists<Account>(offerer_address);
        ensures signer::address_of(result) == offerer_address;
    }

    spec schema AccountContainsAddr {
        account: signer;
        address: address;
        let addr = signer::address_of(account);
        let account_resource = global<Account>(address);
        aborts_if !exists<Account>(address);
        /// [create_signer::high-level-spec-3]
        aborts_if !option::spec_contains(account_resource.signer_capability_offer.for,addr);
    }

    /// The Account existed under the signer
    /// The value of signer_capability_offer.for of Account resource under the signer is to_be_revoked_address
    spec create_resource_address(source: &address, seed: vector<u8>): address {
        pragma opaque;
        pragma aborts_if_is_strict = false;
        // This function should not abort assuming the result of `sha3_256` is deserializable into an address.
        aborts_if [abstract] false;
        ensures [abstract] result == spec_create_resource_address(source, seed);
        ensures [abstract] source != result; // We can assume that the derived resource account does not equal to `source`
    }

    spec fun spec_create_resource_address(source: address, seed: vector<u8>): address;

    spec create_resource_account(source: &signer, seed: vector<u8>): (signer, SignerCapability) {
        pragma aborts_if_is_partial;
        let source_addr = signer::address_of(source);
        let resource_addr = spec_create_resource_address(source_addr, seed);

        aborts_if len(ZERO_AUTH_KEY) != 32;
        include spec_exists_at(resource_addr) ==> CreateResourceAccountAbortsIf;
        include !spec_exists_at(resource_addr) ==> CreateAccountAbortsIf {addr: resource_addr};

        ensures signer::address_of(result_1) == resource_addr;
        let post offer_for = global<Account>(resource_addr).signer_capability_offer.for;
        ensures option::borrow(offer_for) == resource_addr;
        ensures result_2 == SignerCapability { account: resource_addr };
    }

    /// Check if the bytes of the new address is 32.
    /// The Account does not exist under the new address before creating the account.
    /// The system reserved addresses is @0x1 / @0x2 / @0x3 / @0x4 / @0x5  / @0x6 / @0x7 / @0x8 / @0x9 / @0xa.
    spec create_framework_reserved_account(addr: address): (signer, SignerCapability) {
        aborts_if spec_is_framework_address(addr);
        include CreateAccountAbortsIf {addr};
        ensures signer::address_of(result_1) == addr;
        ensures result_2 == SignerCapability { account: addr };
    }

    spec fun spec_is_framework_address(addr: address): bool{
        addr != @0x1 &&
        addr != @0x2 &&
        addr != @0x3 &&
        addr != @0x4 &&
        addr != @0x5 &&
        addr != @0x6 &&
        addr != @0x7 &&
        addr != @0x8 &&
        addr != @0x9 &&
        addr != @0xa
    }

    /// The Account existed under the signer.
    spec create_guid(account_signer: &signer): guid::GUID {
        pragma opaque;
        let addr = signer::address_of(account_signer);
        let account_exists_pre = exists<Account>(addr);
        let pre_gcn = if (account_exists_pre) {
            global<Account>(addr).guid_creation_num
        } else {
            2
        };
        include NewEventHandleAbortsIf { account: account_signer };
        modifies global<Account>(addr);
        ensures exists<Account>(addr);
        /// [high-level-req-11]
        ensures global<Account>(addr).guid_creation_num == pre_gcn + 1;
        ensures result == guid::GUID { id: guid::ID { creation_num: pre_gcn, addr } };
    }

    spec new_event_handle<T: drop + store>(account: &signer): EventHandle<T> {
        pragma opaque;
        let addr = signer::address_of(account);
        let account_exists_pre = exists<Account>(addr);
        let pre_gcn = if (account_exists_pre) {
            global<Account>(addr).guid_creation_num
        } else {
            2
        };
        include NewEventHandleAbortsIf;
        modifies global<Account>(addr);
        ensures exists<Account>(addr);
        ensures global<Account>(addr).guid_creation_num == pre_gcn + 1;
        ensures result.counter == 0;
        ensures result.guid == guid::GUID { id: guid::ID { creation_num: pre_gcn, addr } };
    }

    spec schema EnsureResourceExistsAbortsIf {
        use std::features::{Self, DEFAULT_ACCOUNT_RESOURCE};
        addr: address;
        let feature_on = features::spec_is_enabled(DEFAULT_ACCOUNT_RESOURCE);
        let account_exists_pre = exists<Account>(addr);
        aborts_if !account_exists_pre && !feature_on;
        aborts_if !account_exists_pre && feature_on
            && (addr == @vm_reserved || addr == @aptos_framework || addr == @aptos_token);
        aborts_if !account_exists_pre && feature_on
            && len(bcs::to_bytes(addr)) != 32;
    }

    spec schema AccountPermissionAbortsIf<Perm> {
        use aptos_framework::permissioned_signer;
        account: &signer;
        perm: Perm;
        aborts_if permissioned_signer::spec_is_permissioned_signer(account)
            && !exists<permissioned_signer::PermissionStorage>(permissioned_signer::spec_permission_address(account));
        aborts_if !permissioned_signer::spec_check_permission_exists(account, perm);
    }

    spec schema NewEventHandleAbortsIf {
        use std::features::{Self, DEFAULT_ACCOUNT_RESOURCE};
        account: &signer;
        let addr = signer::address_of(account);
        let feature_on = features::spec_is_enabled(DEFAULT_ACCOUNT_RESOURCE);
        let account_exists_pre = exists<Account>(addr);
        // Feature OFF (legacy): missing Account aborts.
        aborts_if !account_exists_pre && !feature_on;
        // Feature ON: missing Account at a reserved address aborts in create_account_if_does_not_exist.
        aborts_if !account_exists_pre && feature_on
            && (addr == @vm_reserved || addr == @aptos_framework || addr == @aptos_token);
        // Vacuous in practice (addresses are 32 bytes) but the prover can't see through bcs::to_bytes.
        aborts_if !account_exists_pre && feature_on
            && len(bcs::to_bytes(addr)) != 32;
        // Pre-existing account uses its current counter; freshly-created has guid_creation_num == 2
        // (create_account_unchecked initializes via 2 built-in event handles).
        let creation_num = if (account_exists_pre) {
            global<Account>(addr).guid_creation_num
        } else {
            2
        };
        aborts_if creation_num + 1 > MAX_U64;
        aborts_if creation_num + 1 >= MAX_GUID_CREATION_NUM;
    }

    spec register_coin<CoinType>(account_addr: address) {
        aborts_if !type_info::spec_is_struct<CoinType>();
    }

    spec create_signer_with_capability(capability: &SignerCapability): signer {
        let addr = capability.account;
        ensures signer::address_of(result) == addr;
    }

    spec schema CreateResourceAccountAbortsIf {
        resource_addr: address;
        let account = global<Account>(resource_addr);
        aborts_if exists<Account>(resource_addr)
            && option::spec_is_some(account.signer_capability_offer.for);
        aborts_if exists<Account>(resource_addr) && account.sequence_number != 0;
    }

    spec originating_address(auth_key: address): Option<address> {
        aborts_if !exists<OriginatingAddress>(@aptos_framework);
        let address_map = global<OriginatingAddress>(@aptos_framework).address_map;
        ensures table::spec_contains(address_map, auth_key) ==>
            result == option::spec_some(table::spec_get(address_map, auth_key));
        ensures !table::spec_contains(address_map, auth_key) ==>
            result == option::spec_none<address>();
    }

    spec update_auth_key_and_originating_address_table(
        originating_addr: address,
        account_resource: &mut Account,
        new_auth_key_vector: vector<u8>,
    ) {
        modifies global<OriginatingAddress>(@aptos_framework);
        include UpdateAuthKeyAndOriginatingAddressTableAbortsIf {
            current_authentication_key_bytes: account_resource.authentication_key,
        };
    }
    spec schema UpdateAuthKeyAndOriginatingAddressTableAbortsIf {
        originating_addr: address;
        current_authentication_key_bytes: vector<u8>;
        new_auth_key_vector: vector<u8>;
        let address_map = global<OriginatingAddress>(@aptos_framework).address_map;
        let curr_auth_key = from_bcs::deserialize<address>(current_authentication_key_bytes);
        let new_auth_key = from_bcs::deserialize<address>(new_auth_key_vector);
        aborts_if !exists<OriginatingAddress>(@aptos_framework);
        aborts_if !from_bcs::deserializable<address>(current_authentication_key_bytes);
        aborts_if table::spec_contains(address_map, curr_auth_key) &&
            table::spec_get(address_map, curr_auth_key) != originating_addr;
        aborts_if !from_bcs::deserializable<address>(new_auth_key_vector);
        aborts_if curr_auth_key == new_auth_key;
        aborts_if curr_auth_key != new_auth_key && table::spec_contains(address_map, new_auth_key);

        ensures table::spec_contains(global<OriginatingAddress>(@aptos_framework).address_map, from_bcs::deserialize<address>(new_auth_key_vector));
    }

    spec verify_signed_message<T: drop>(
        account: address,
        account_scheme: u8,
        account_public_key: vector<u8>,
        signed_message_bytes: vector<u8>,
        message: T,
    ) {
        pragma aborts_if_is_partial;

        modifies global<Account>(account);
        aborts_if !spec_exists_at(account);
        let auth_key = spec_get_authentication_key(account);

        include account_scheme == ED25519_SCHEME ==> ed25519::NewUnvalidatedPublicKeyFromBytesAbortsIf { bytes: account_public_key };
        aborts_if account_scheme == ED25519_SCHEME &&
            auth_key != ed25519::spec_public_key_bytes_to_authentication_key(account_public_key);

        include account_scheme == MULTI_ED25519_SCHEME ==> multi_ed25519::NewUnvalidatedPublicKeyFromBytesAbortsIf { bytes: account_public_key };
        aborts_if account_scheme == MULTI_ED25519_SCHEME &&
            auth_key != multi_ed25519::spec_public_key_bytes_to_authentication_key(account_public_key);

        include account_scheme == ED25519_SCHEME ==> ed25519::NewSignatureFromBytesAbortsIf { bytes: signed_message_bytes };
        include account_scheme == MULTI_ED25519_SCHEME ==> multi_ed25519::NewSignatureFromBytesAbortsIf { bytes: signed_message_bytes };

        aborts_if account_scheme != ED25519_SCHEME && account_scheme != MULTI_ED25519_SCHEME;
    }

    spec set_originating_address(_account: &signer) {
        aborts_if true;
    }

    spec upsert_ed25519_backup_key_on_keyless_account {
        pragma verify = false;
    }

    spec upsert_ed25519_backup_key_and_encrypt_dk {
        pragma verify = false;
    }
}
