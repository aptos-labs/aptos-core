spec aptos_framework::account {
    
    spec module {
        pragma verify = true;
        pragma aborts_if_is_strict;
    }

    spec create_signer {
        pragma opaque;
        aborts_if [abstract] false;
        ensures [abstract] signer::address_of(result) == addr;
    }

    spec initialize {
        aborts_if !system_addresses::is_aptos_framework_address(signer::address_of(aptos_framework));
        aborts_if exists<OriginatingAddress>(signer::address_of(aptos_framework));
        ensures exists<OriginatingAddress>(signer::address_of(aptos_framework));
    }

    spec create_account {
        pragma aborts_if_is_partial;
        aborts_if exists<Account>(new_address);
        aborts_if new_address == @vm_reserved || new_address == @aptos_framework || new_address == @aptos_token;
    }

    spec create_account_unchecked {
        pragma aborts_if_is_partial;
        let authentication_key = bcs::to_bytes(new_address);
        aborts_if vector::length(authentication_key) != 32;
        ensures signer::address_of(result) == new_address;
    }

    spec get_guid_next_creation_num {
        aborts_if !exists<Account>(addr);
        ensures result == global<Account>(addr).guid_creation_num;
    }

    spec get_sequence_number {
        aborts_if !exists<Account>(addr);
        ensures result == global<Account>(addr).sequence_number;
    }

    spec increment_sequence_number {
        let post adr_post = global<Account>(addr);
        let adr = global<Account>(addr);
        let sequence = adr.sequence_number;
        aborts_if !exists<Account>(addr);
        aborts_if sequence >= MAX_U64;
        aborts_if sequence + 1 > MAX_U128;
        ensures adr_post.sequence_number == adr.sequence_number + 1;
    }

    spec get_authentication_key {
        aborts_if !exists<Account>(addr);
        ensures result == global<Account>(addr).authentication_key;
    }

    spec rotate_authentication_key_internal {
        let addr = signer::address_of(account);
        let account_resource = global<Account>(addr);
        aborts_if !exists<Account>(addr);
        aborts_if vector::length(new_auth_key) != 32;
    }

    spec assert_valid_signature_and_get_auth_key {
        pragma aborts_if_is_partial;
        aborts_if scheme != ED25519_SCHEME && scheme != MULTI_ED25519_SCHEME;
    }

    spec rotate_authentication_key {
        pragma aborts_if_is_partial;
        let addr = signer::address_of(account);
        let account_resource = global<Account>(addr);
        aborts_if !exists<Account>(addr);
        aborts_if from_scheme != ED25519_SCHEME && from_scheme != MULTI_ED25519_SCHEME;
    }

    spec offer_signer_capability {
        pragma aborts_if_is_partial;
        let source_address = signer::address_of(account);
        aborts_if !exists<Account>(recipient_address);
        aborts_if account_scheme != ED25519_SCHEME && account_scheme != MULTI_ED25519_SCHEME;
    }

    spec revoke_signer_capability {
        pragma aborts_if_is_partial;
        aborts_if !exists<Account>(to_be_revoked_address);
        ensures exists<Account>(to_be_revoked_address);
    }

    spec create_authorized_signer {
        pragma aborts_if_is_partial;
        let account_resource = global<Account>(offerer_address);
        aborts_if !exists<Account>(offerer_address);
        ensures exists<Account>(offerer_address);
        ensures signer::address_of(result) == offerer_address;
    }

    spec create_resource_address {
        pragma verify = false;
    }

    spec create_resource_account {
        pragma verify = false;
    }

    spec create_framework_reserved_account {
        pragma aborts_if_is_partial;
        aborts_if addr != @0x1 &&
                addr != @0x2 &&
                addr != @0x3 &&
                addr != @0x4 &&
                addr != @0x5 &&
                addr != @0x6 &&
                addr != @0x7 &&
                addr != @0x8 &&
                addr != @0x9 &&
                addr != @0xa;
        ensures result_2 == SignerCapability { account: addr };
    }

    spec create_guid {
        let addr = signer::address_of(account_signer);
        let account = global<Account>(addr);
        aborts_if !exists<Account>(addr);
        aborts_if account.guid_creation_num + 1 > MAX_U64;
    }

    spec new_event_handle {
        let addr = signer::address_of(account);
        let account = global<Account>(addr);
        aborts_if !exists<Account>(addr);
        aborts_if account.guid_creation_num + 1 > MAX_U64;
    }

    spec register_coin {
        aborts_if !exists<Account>(account_addr);
    }

    spec create_signer_with_capability {
        let addr = capability.account;
        ensures signer::address_of(result) == addr;
    }

}
