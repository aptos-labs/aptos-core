spec aptos_framework::account {
    spec module {
        pragma verify = true;
        pragma aborts_if_is_strict;
    }

    /// Only the address `@aptos_framework` can call.
    /// OriginatingAddress does not exist under `@aptos_framework` before the call.
    spec initialize(aptos_framework: &signer) {
        let aptos_addr = signer::address_of(aptos_framework);
        aborts_if !system_addresses::is_aptos_framework_address(aptos_addr);
        aborts_if exists<OriginatingAddress>(aptos_addr);
        ensures exists<OriginatingAddress>(aptos_addr);
    }

    /// Check if the bytes of the new address is 32.
    /// The Account does not exist under the new address before creating the account.
    /// Limit the new account address is not @vm_reserved / @aptos_framework / @aptos_toke.
    spec create_account(new_address: address): signer {
        include CreateAccountAbortsIf {addr: new_address};
        aborts_if new_address == @vm_reserved || new_address == @aptos_framework || new_address == @aptos_token;
        ensures signer::address_of(result) == new_address;
    }

    /// Check if the bytes of the new address is 32.
    /// The Account does not exist under the new address before creating the account.
    spec create_account_unchecked(new_address: address): signer {
        include CreateAccountAbortsIf {addr: new_address};
        ensures signer::address_of(result) == new_address;
    }

    spec schema CreateAccountAbortsIf {
        addr: address;
        let authentication_key = bcs::to_bytes(addr);
        aborts_if len(authentication_key) != 32;
        aborts_if exists<Account>(addr);
    }

    spec get_guid_next_creation_num(addr: address): u64 {
        aborts_if !exists<Account>(addr);
        ensures result == global<Account>(addr).guid_creation_num;
    }

    spec get_sequence_number(addr: address): u64 {
        aborts_if !exists<Account>(addr);
        ensures result == global<Account>(addr).sequence_number;
    }

    /// The Account existed under the address.
    /// The sequence_number of the Account is up to MAX_U64.
    spec increment_sequence_number(addr: address) {
        let sequence_number = global<Account>(addr).sequence_number;
        aborts_if !exists<Account>(addr);
        aborts_if sequence_number == MAX_U64;
        modifies global<Account>(addr);
        let post post_sequence_number = global<Account>(addr).sequence_number;
        ensures post_sequence_number == sequence_number + 1;
    }

    spec get_authentication_key(addr: address): vector<u8> {
        aborts_if !exists<Account>(addr);
        ensures result == global<Account>(addr).authentication_key;
    }

    /// The Account existed under the signer before the call.
    /// The length of new_auth_key is 32.
    spec rotate_authentication_key_internal(account: &signer, new_auth_key: vector<u8>) {
        let addr = signer::address_of(account);
        let post account_resource = global<Account>(addr);
        aborts_if !exists<Account>(addr);
        aborts_if vector::length(new_auth_key) != 32;
        modifies global<Account>(addr);
        ensures account_resource.authentication_key == new_auth_key;
    }

    spec fun spec_assert_valid_rotation_proof_signature_and_get_auth_key(scheme: u8, public_key_bytes: vector<u8>, signature: vector<u8>, challenge: RotationProofChallenge): vector<u8>;

    spec assert_valid_rotation_proof_signature_and_get_auth_key(scheme: u8, public_key_bytes: vector<u8>, signature: vector<u8>, challenge: &RotationProofChallenge): vector<u8> {
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
        let addr = signer::address_of(account);
        let account_resource = global<Account>(addr);
        aborts_if !exists<Account>(addr);

        include from_scheme == ED25519_SCHEME ==> ed25519::NewUnvalidatedPublicKeyFromBytesAbortsIf { bytes: from_public_key_bytes };
        aborts_if from_scheme == ED25519_SCHEME && ({
            let expected_auth_key = ed25519::spec_public_key_bytes_to_authentication_key(from_public_key_bytes);
            account_resource.authentication_key != expected_auth_key
        });
        include from_scheme == MULTI_ED25519_SCHEME ==> multi_ed25519::NewUnvalidatedPublicKeyFromBytesAbortsIf { bytes: from_public_key_bytes };
        aborts_if from_scheme == MULTI_ED25519_SCHEME && ({
            let from_auth_key = multi_ed25519::spec_public_key_bytes_to_authentication_key(from_public_key_bytes);
            account_resource.authentication_key != from_auth_key
        });
        aborts_if from_scheme != ED25519_SCHEME && from_scheme != MULTI_ED25519_SCHEME;

        let curr_auth_key = from_bcs::deserialize<address>(account_resource.authentication_key);
        aborts_if !from_bcs::deserializable<address>(account_resource.authentication_key);

        let challenge = RotationProofChallenge {
            sequence_number: account_resource.sequence_number,
            originator: addr,
            current_auth_key: curr_auth_key,
            new_public_key: to_public_key_bytes,
        };

        include AssertValidRotationProofSignatureAndGetAuthKeyAbortsIf {
            scheme: from_scheme,
            public_key_bytes: from_public_key_bytes,
            signature: cap_rotate_key,
            challenge: challenge,
        };

        include AssertValidRotationProofSignatureAndGetAuthKeyAbortsIf {
            scheme: to_scheme,
            public_key_bytes: to_public_key_bytes,
            signature: cap_update_table,
            challenge: challenge,
        };

        // Verify all properties in update_auth_key_and_originating_address_table
        let originating_addr = addr;
        let new_auth_key_vector = spec_assert_valid_rotation_proof_signature_and_get_auth_key(to_scheme, to_public_key_bytes, cap_update_table, challenge);

        let address_map = global<OriginatingAddress>(@aptos_framework).address_map;
        let new_auth_key = from_bcs::deserialize<address>(new_auth_key_vector);

        aborts_if !exists<OriginatingAddress>(@aptos_framework);
        aborts_if !from_bcs::deserializable<address>(account_resource.authentication_key);
        aborts_if table::spec_contains(address_map, curr_auth_key) &&
            table::spec_get(address_map, curr_auth_key) != originating_addr;

        aborts_if !from_bcs::deserializable<address>(new_auth_key_vector);

        aborts_if curr_auth_key != new_auth_key && table::spec_contains(address_map, new_auth_key);

    }

    spec rotate_authentication_key_with_rotation_capability(
        delegate_signer: &signer,
        rotation_cap_offerer_address: address,
        new_scheme: u8,
        new_public_key_bytes: vector<u8>,
        cap_update_table: vector<u8>
    ) {
        aborts_if !exists<Account>(rotation_cap_offerer_address);
        let delegate_address = signer::address_of(delegate_signer);
        let offerer_account_resource = global<Account>(rotation_cap_offerer_address);
        aborts_if !from_bcs::deserializable<address>(offerer_account_resource.authentication_key);
        let curr_auth_key = from_bcs::deserialize<address>(offerer_account_resource.authentication_key);
        aborts_if !exists<Account>(delegate_address);
        let challenge = RotationProofChallenge {
            sequence_number: global<Account>(delegate_address).sequence_number,
            originator: rotation_cap_offerer_address,
            current_auth_key: curr_auth_key,
            new_public_key: new_public_key_bytes,
        };
        aborts_if !option::spec_contains(offerer_account_resource.rotation_capability_offer.for, delegate_address);
        include AssertValidRotationProofSignatureAndGetAuthKeyAbortsIf {
            scheme: new_scheme,
            public_key_bytes: new_public_key_bytes,
            signature: cap_update_table,
            challenge: challenge,
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
    }

    spec offer_rotation_capability(
        account: &signer,
        rotation_capability_sig_bytes: vector<u8>,
        account_scheme: u8,
        account_public_key_bytes: vector<u8>,
        recipient_address: address,
    ) {
        let source_address = signer::address_of(account);
        let account_resource = global<Account>(source_address);
        let proof_challenge = RotationCapabilityOfferProofChallengeV2 {
            chain_id: global<chain_id::ChainId>(@aptos_framework).id,
            sequence_number: account_resource.sequence_number,
            source_address,
            recipient_address,
        };

        aborts_if !exists<chain_id::ChainId>(@aptos_framework);
        aborts_if !exists<Account>(recipient_address);
        aborts_if !exists<Account>(source_address);

        include account_scheme == ED25519_SCHEME ==> ed25519::NewUnvalidatedPublicKeyFromBytesAbortsIf { bytes: account_public_key_bytes };
        aborts_if account_scheme == ED25519_SCHEME && ({
            let expected_auth_key = ed25519::spec_public_key_bytes_to_authentication_key(account_public_key_bytes);
            account_resource.authentication_key != expected_auth_key
        });
        include account_scheme == ED25519_SCHEME ==> ed25519::NewSignatureFromBytesAbortsIf { bytes: rotation_capability_sig_bytes };
        aborts_if account_scheme == ED25519_SCHEME && !ed25519::spec_signature_verify_strict_t(
            ed25519::Signature { bytes: rotation_capability_sig_bytes },
            ed25519::UnvalidatedPublicKey { bytes: account_public_key_bytes },
            proof_challenge
        );

        include account_scheme == MULTI_ED25519_SCHEME ==> multi_ed25519::NewUnvalidatedPublicKeyFromBytesAbortsIf { bytes: account_public_key_bytes };
        aborts_if account_scheme == MULTI_ED25519_SCHEME && ({
            let expected_auth_key = multi_ed25519::spec_public_key_bytes_to_authentication_key(account_public_key_bytes);
            account_resource.authentication_key != expected_auth_key
        });
        include account_scheme == MULTI_ED25519_SCHEME ==> multi_ed25519::NewSignatureFromBytesAbortsIf { bytes: rotation_capability_sig_bytes };
        aborts_if account_scheme == MULTI_ED25519_SCHEME && !multi_ed25519::spec_signature_verify_strict_t(
            multi_ed25519::Signature { bytes: rotation_capability_sig_bytes },
            multi_ed25519::UnvalidatedPublicKey { bytes: account_public_key_bytes },
            proof_challenge
        );

        aborts_if account_scheme != ED25519_SCHEME && account_scheme != MULTI_ED25519_SCHEME;

        modifies global<Account>(source_address);
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
        let source_address = signer::address_of(account);
        let account_resource = global<Account>(source_address);
        let proof_challenge = SignerCapabilityOfferProofChallengeV2 {
            sequence_number: account_resource.sequence_number,
            source_address,
            recipient_address,
        };

        aborts_if !exists<Account>(recipient_address);
        aborts_if !exists<Account>(source_address);

        include account_scheme == ED25519_SCHEME ==> ed25519::NewUnvalidatedPublicKeyFromBytesAbortsIf { bytes: account_public_key_bytes };
        aborts_if account_scheme == ED25519_SCHEME && ({
            let expected_auth_key = ed25519::spec_public_key_bytes_to_authentication_key(account_public_key_bytes);
            account_resource.authentication_key != expected_auth_key
        });
        include account_scheme == ED25519_SCHEME ==> ed25519::NewSignatureFromBytesAbortsIf { bytes: signer_capability_sig_bytes };
        aborts_if account_scheme == ED25519_SCHEME && !ed25519::spec_signature_verify_strict_t(
            ed25519::Signature { bytes: signer_capability_sig_bytes },
            ed25519::UnvalidatedPublicKey { bytes: account_public_key_bytes },
            proof_challenge
        );

        include account_scheme == MULTI_ED25519_SCHEME ==> multi_ed25519::NewUnvalidatedPublicKeyFromBytesAbortsIf { bytes: account_public_key_bytes };
        aborts_if account_scheme == MULTI_ED25519_SCHEME && ({
            let expected_auth_key = multi_ed25519::spec_public_key_bytes_to_authentication_key(account_public_key_bytes);
            account_resource.authentication_key != expected_auth_key
        });
        include account_scheme == MULTI_ED25519_SCHEME ==> multi_ed25519::NewSignatureFromBytesAbortsIf { bytes: signer_capability_sig_bytes };
        aborts_if account_scheme == MULTI_ED25519_SCHEME && !multi_ed25519::spec_signature_verify_strict_t(
            multi_ed25519::Signature { bytes: signer_capability_sig_bytes },
            multi_ed25519::UnvalidatedPublicKey { bytes: account_public_key_bytes },
            proof_challenge
        );

        aborts_if account_scheme != ED25519_SCHEME && account_scheme != MULTI_ED25519_SCHEME;

        modifies global<Account>(source_address);
    }

    spec is_signer_capability_offered(account_addr: address): bool {
        aborts_if !exists<Account>(account_addr);
    }

    spec get_signer_capability_offer_for(account_addr: address): address {
        aborts_if !exists<Account>(account_addr);
        let account_resource = global<Account>(account_addr);
        aborts_if len(account_resource.signer_capability_offer.for.vec) == 0;
    }

    /// The Account existed under the signer.
    /// The value of signer_capability_offer.for of Account resource under the signer is to_be_revoked_address.
    spec revoke_signer_capability(account: &signer, to_be_revoked_address: address) {
        aborts_if !exists<Account>(to_be_revoked_address);
        let addr = signer::address_of(account);
        let account_resource = global<Account>(addr);
        aborts_if !exists<Account>(addr);
        aborts_if !option::spec_contains(account_resource.signer_capability_offer.for,to_be_revoked_address);
        modifies global<Account>(addr);
        ensures exists<Account>(to_be_revoked_address);
    }

    spec revoke_any_signer_capability(account: &signer) {
        modifies global<Account>(signer::address_of(account));
        aborts_if !exists<Account>(signer::address_of(account));
        let account_resource = global<Account>(signer::address_of(account));
        aborts_if !option::is_some(account_resource.signer_capability_offer.for);
    }

    spec revoke_rotation_capability(account: &signer, to_be_revoked_address: address) {
        aborts_if !exists<Account>(to_be_revoked_address);
        let addr = signer::address_of(account);
        let account_resource = global<Account>(addr);
        aborts_if !exists<Account>(addr);
        aborts_if !option::spec_contains(account_resource.rotation_capability_offer.for,to_be_revoked_address);
        modifies global<Account>(addr);
        ensures exists<Account>(to_be_revoked_address);
    }

    spec revoke_any_rotation_capability(account: &signer) {
        modifies global<Account>(signer::address_of(account));
        aborts_if !exists<Account>(signer::address_of(account));
        let account_resource = global<Account>(signer::address_of(account));
        aborts_if !option::is_some(account_resource.rotation_capability_offer.for);
    }

    /// The Account existed under the signer.
    /// The value of signer_capability_offer.for of Account resource under the signer is offerer_address.
    spec create_authorized_signer(account: &signer, offerer_address: address): signer {
        include AccountContainsAddr{
            account: account,
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
    }

    spec fun spec_create_resource_address(source: address, seed: vector<u8>): address;

    spec create_resource_account(source: &signer, seed: vector<u8>): (signer, SignerCapability) {
        let source_addr = signer::address_of(source);
        let resource_addr = spec_create_resource_address(source_addr, seed);

        aborts_if len(ZERO_AUTH_KEY) != 32;
        include exists_at(resource_addr) ==> CreateResourceAccountAbortsIf;
        include !exists_at(resource_addr) ==> CreateAccountAbortsIf {addr: resource_addr};
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
    /// The guid_creation_num of the ccount resource is up to MAX_U64.
    spec create_guid(account_signer: &signer): guid::GUID {
        let addr = signer::address_of(account_signer);
        include NewEventHandleAbortsIf {
            account: account_signer,
        };
        modifies global<Account>(addr);
    }

    /// The Account existed under the signer.
    /// The guid_creation_num of the Account is up to MAX_U64.
    spec new_event_handle<T: drop + store>(account: &signer): EventHandle<T> {
        include NewEventHandleAbortsIf;
    }
    spec schema NewEventHandleAbortsIf {
        account: &signer;
        let addr = signer::address_of(account);
        let account = global<Account>(addr);
        aborts_if !exists<Account>(addr);
        aborts_if account.guid_creation_num + 1 > MAX_U64;
        aborts_if account.guid_creation_num + 1 >= MAX_GUID_CREATION_NUM;
    }

    spec register_coin<CoinType>(account_addr: address) {
        aborts_if !exists<Account>(account_addr);
        aborts_if !type_info::spec_is_struct<CoinType>();
        modifies global<Account>(account_addr);
    }

    spec create_signer_with_capability(capability: &SignerCapability): signer {
        let addr = capability.account;
        ensures signer::address_of(result) == addr;
    }

    spec schema CreateResourceAccountAbortsIf {
        resource_addr: address;
        let account = global<Account>(resource_addr);
        aborts_if len(account.signer_capability_offer.for.vec) != 0;
        aborts_if account.sequence_number != 0;
    }

    spec update_auth_key_and_originating_address_table(
        originating_addr: address,
        account_resource: &mut Account,
        new_auth_key_vector: vector<u8>,
    ) {
        modifies global<OriginatingAddress>(@aptos_framework);
        include UpdateAuthKeyAndOriginatingAddressTableAbortsIf;
    }
    spec schema UpdateAuthKeyAndOriginatingAddressTableAbortsIf {
        originating_addr: address;
        account_resource: Account;
        new_auth_key_vector: vector<u8>;
        let address_map = global<OriginatingAddress>(@aptos_framework).address_map;
        let curr_auth_key = from_bcs::deserialize<address>(account_resource.authentication_key);
        let new_auth_key = from_bcs::deserialize<address>(new_auth_key_vector);
        aborts_if !exists<OriginatingAddress>(@aptos_framework);
        aborts_if !from_bcs::deserializable<address>(account_resource.authentication_key);
        aborts_if table::spec_contains(address_map, curr_auth_key) &&
            table::spec_get(address_map, curr_auth_key) != originating_addr;
        aborts_if !from_bcs::deserializable<address>(new_auth_key_vector);
        aborts_if curr_auth_key != new_auth_key && table::spec_contains(address_map, new_auth_key);
    }

    spec verify_signed_message<T: drop>(
        account: address,
        account_scheme: u8,
        account_public_key: vector<u8>,
        signed_message_bytes: vector<u8>,
        message: T,
    ) {
        pragma verify = false;
        modifies global<Account>(account);
    }
}
