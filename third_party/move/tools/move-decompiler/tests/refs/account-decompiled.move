module 0x1::account {
    struct Account has store, key {
        authentication_key: vector<u8>,
        sequence_number: u64,
        guid_creation_num: u64,
        coin_register_events: 0x1::event::EventHandle<CoinRegisterEvent>,
        key_rotation_events: 0x1::event::EventHandle<KeyRotationEvent>,
        rotation_capability_offer: CapabilityOffer<RotationCapability>,
        signer_capability_offer: CapabilityOffer<SignerCapability>,
    }
    
    struct CapabilityOffer<phantom T0> has store {
        for: 0x1::option::Option<address>,
    }
    
    struct CoinRegisterEvent has drop, store {
        type_info: 0x1::type_info::TypeInfo,
    }
    
    struct KeyRotation has drop, store {
        account: address,
        old_authentication_key: vector<u8>,
        new_authentication_key: vector<u8>,
    }
    
    struct KeyRotationEvent has drop, store {
        old_authentication_key: vector<u8>,
        new_authentication_key: vector<u8>,
    }
    
    struct OriginatingAddress has key {
        address_map: 0x1::table::Table<address, address>,
    }
    
    struct RotationCapability has drop, store {
        account: address,
    }
    
    struct RotationCapabilityOfferProofChallenge has drop {
        sequence_number: u64,
        recipient_address: address,
    }
    
    struct RotationCapabilityOfferProofChallengeV2 has drop {
        chain_id: u8,
        sequence_number: u64,
        source_address: address,
        recipient_address: address,
    }
    
    struct RotationProofChallenge has copy, drop {
        sequence_number: u64,
        originator: address,
        current_auth_key: address,
        new_public_key: vector<u8>,
    }
    
    struct SignerCapability has drop, store {
        account: address,
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
    
    public fun new_event_handle<T0: drop + store>(arg0: &signer) : 0x1::event::EventHandle<T0> acquires Account {
        let v0 = create_guid(arg0);
        0x1::event::new_event_handle<T0>(v0)
    }
    
    fun assert_valid_rotation_proof_signature_and_get_auth_key(arg0: u8, arg1: vector<u8>, arg2: vector<u8>, arg3: &RotationProofChallenge) : vector<u8> {
        if (arg0 == 0) {
            let v1 = 0x1::ed25519::new_unvalidated_public_key_from_bytes(arg1);
            let v2 = 0x1::ed25519::new_signature_from_bytes(arg2);
            let v3 = 0x1::ed25519::signature_verify_strict_t<RotationProofChallenge>(&v2, &v1, *arg3);
            assert!(v3, 0x1::error::invalid_argument(8));
            0x1::ed25519::unvalidated_public_key_to_authentication_key(&v1)
        } else {
            assert!(arg0 == 1, 0x1::error::invalid_argument(12));
            let v4 = 0x1::multi_ed25519::new_unvalidated_public_key_from_bytes(arg1);
            let v5 = 0x1::multi_ed25519::new_signature_from_bytes(arg2);
            let v6 = 0x1::multi_ed25519::signature_verify_strict_t<RotationProofChallenge>(&v5, &v4, *arg3);
            assert!(v6, 0x1::error::invalid_argument(8));
            0x1::multi_ed25519::unvalidated_public_key_to_authentication_key(&v4)
        }
    }
    
    public(friend) fun create_account(arg0: address) : signer {
        assert!(!exists<Account>(arg0), 0x1::error::already_exists(1));
        assert!(arg0 != @0x3001 && arg0 != @0x1 && arg0 != @0x1337, 0x1::error::invalid_argument(5));
        create_account_unchecked(arg0)
    }
    
    fun create_account_if_does_not_exist(arg0: address) {
        if (!exists<Account>(arg0)) {
            create_account(arg0);
        };
    }
    
    fun create_account_unchecked(arg0: address) : signer {
        let v0 = 0x1::create_signer::create_signer(arg0);
        let v1 = 0x1::bcs::to_bytes<address>(&arg0);
        assert!(0x1::vector::length<u8>(&v1) == 32, 0x1::error::invalid_argument(4));
        let v2 = 0;
        let v3 = 0x1::event::new_event_handle<CoinRegisterEvent>(0x1::guid::create(arg0, &mut v2));
        let v4 = 0x1::event::new_event_handle<KeyRotationEvent>(0x1::guid::create(arg0, &mut v2));
        let v5 = CapabilityOffer<RotationCapability>{for: 0x1::option::none<address>()};
        let v6 = CapabilityOffer<SignerCapability>{for: 0x1::option::none<address>()};
        let v7 = Account{
            authentication_key        : v1, 
            sequence_number           : 0, 
            guid_creation_num         : v2, 
            coin_register_events      : v3, 
            key_rotation_events       : v4, 
            rotation_capability_offer : v5, 
            signer_capability_offer   : v6,
        };
        move_to<Account>(&v0, v7);
        v0
    }
    
    public fun create_authorized_signer(arg0: &signer, arg1: address) : signer acquires Account {
        assert!(exists_at(arg1), 0x1::error::not_found(17));
        let v0 = 0x1::signer::address_of(arg0);
        let v1 = 0x1::option::contains<address>(&borrow_global<Account>(arg1).signer_capability_offer.for, &v0);
        assert!(v1, 0x1::error::not_found(14));
        0x1::create_signer::create_signer(arg1)
    }
    
    public(friend) fun create_framework_reserved_account(arg0: address) : (signer, SignerCapability) {
        assert!(arg0 == @0x1 || arg0 == @0x2 || arg0 == @0x3 || arg0 == @0x4 || arg0 == @0x5 || arg0 == @0x6 || arg0 == @0x7 || arg0 == @0x8 || arg0 == @0x9 || arg0 == @0xa, 0x1::error::permission_denied(11));
        let v0 = SignerCapability{account: arg0};
        (create_account_unchecked(arg0), v0)
    }
    
    public fun create_guid(arg0: &signer) : 0x1::guid::GUID acquires Account {
        let v0 = 0x1::signer::address_of(arg0);
        let v1 = borrow_global_mut<Account>(v0);
        assert!(v1.guid_creation_num < 1125899906842624, 0x1::error::out_of_range(20));
        0x1::guid::create(v0, &mut v1.guid_creation_num)
    }
    
    public fun create_resource_account(arg0: &signer, arg1: vector<u8>) : (signer, SignerCapability) acquires Account {
        let v0 = 0x1::signer::address_of(arg0);
        let v1 = create_resource_address(&v0, arg1);
        let v2 = if (exists_at(v1)) {
            let v3 = borrow_global<Account>(v1);
            assert!(0x1::option::is_none<address>(&v3.signer_capability_offer.for), 0x1::error::already_exists(15));
            assert!(v3.sequence_number == 0, 0x1::error::invalid_state(16));
            0x1::create_signer::create_signer(v1)
        } else {
            create_account_unchecked(v1)
        };
        let v4 = v2;
        rotate_authentication_key_internal(&v4, x"0000000000000000000000000000000000000000000000000000000000000000");
        borrow_global_mut<Account>(v1).signer_capability_offer.for = 0x1::option::some<address>(v1);
        let v5 = SignerCapability{account: v1};
        (v4, v5)
    }
    
    public fun create_resource_address(arg0: &address, arg1: vector<u8>) : address {
        let v0 = 0x1::bcs::to_bytes<address>(arg0);
        0x1::vector::append<u8>(&mut v0, arg1);
        0x1::vector::push_back<u8>(&mut v0, 255);
        0x1::from_bcs::to_address(0x1::hash::sha3_256(v0))
    }
    
    public fun create_signer_with_capability(arg0: &SignerCapability) : signer {
        0x1::create_signer::create_signer(arg0.account)
    }
    
    public fun exists_at(arg0: address) : bool {
        exists<Account>(arg0)
    }
    
    public fun get_authentication_key(arg0: address) : vector<u8> acquires Account {
        borrow_global<Account>(arg0).authentication_key
    }
    
    public fun get_guid_next_creation_num(arg0: address) : u64 acquires Account {
        borrow_global<Account>(arg0).guid_creation_num
    }
    
    public fun get_rotation_capability_offer_for(arg0: address) : address acquires Account {
        let v0 = borrow_global<Account>(arg0);
        assert!(0x1::option::is_some<address>(&v0.rotation_capability_offer.for), 0x1::error::not_found(19));
        *0x1::option::borrow<address>(&v0.rotation_capability_offer.for)
    }
    
    public fun get_sequence_number(arg0: address) : u64 acquires Account {
        borrow_global<Account>(arg0).sequence_number
    }
    
    public fun get_signer_capability_address(arg0: &SignerCapability) : address {
        arg0.account
    }
    
    public fun get_signer_capability_offer_for(arg0: address) : address acquires Account {
        let v0 = borrow_global<Account>(arg0);
        assert!(0x1::option::is_some<address>(&v0.signer_capability_offer.for), 0x1::error::not_found(19));
        *0x1::option::borrow<address>(&v0.signer_capability_offer.for)
    }
    
    public(friend) fun increment_sequence_number(arg0: address) acquires Account {
        let v0 = &mut borrow_global_mut<Account>(arg0).sequence_number;
        assert!((*v0 as u128) < 18446744073709551615, 0x1::error::out_of_range(3));
        *v0 = *v0 + 1;
    }
    
    public(friend) fun initialize(arg0: &signer) {
        0x1::system_addresses::assert_aptos_framework(arg0);
        let v0 = OriginatingAddress{address_map: 0x1::table::new<address, address>()};
        move_to<OriginatingAddress>(arg0, v0);
    }
    
    public fun is_rotation_capability_offered(arg0: address) : bool acquires Account {
        0x1::option::is_some<address>(&borrow_global<Account>(arg0).rotation_capability_offer.for)
    }
    
    public fun is_signer_capability_offered(arg0: address) : bool acquires Account {
        0x1::option::is_some<address>(&borrow_global<Account>(arg0).signer_capability_offer.for)
    }
    
    public entry fun offer_rotation_capability(arg0: &signer, arg1: vector<u8>, arg2: u8, arg3: vector<u8>, arg4: address) acquires Account {
        let v0 = 0x1::signer::address_of(arg0);
        assert!(exists_at(arg4), 0x1::error::not_found(2));
        let v1 = borrow_global_mut<Account>(v0);
        let v2 = 0x1::chain_id::get();
        let v3 = RotationCapabilityOfferProofChallengeV2{
            chain_id          : v2, 
            sequence_number   : v1.sequence_number, 
            source_address    : v0, 
            recipient_address : arg4,
        };
        if (arg2 == 0) {
            let v4 = 0x1::ed25519::new_unvalidated_public_key_from_bytes(arg3);
            assert!(v1.authentication_key == 0x1::ed25519::unvalidated_public_key_to_authentication_key(&v4), 0x1::error::invalid_argument(7));
            let v5 = 0x1::ed25519::new_signature_from_bytes(arg1);
            assert!(0x1::ed25519::signature_verify_strict_t<RotationCapabilityOfferProofChallengeV2>(&v5, &v4, v3), 0x1::error::invalid_argument(8));
        } else {
            assert!(arg2 == 1, 0x1::error::invalid_argument(12));
            let v6 = 0x1::multi_ed25519::new_unvalidated_public_key_from_bytes(arg3);
            assert!(v1.authentication_key == 0x1::multi_ed25519::unvalidated_public_key_to_authentication_key(&v6), 0x1::error::invalid_argument(7));
            let v7 = 0x1::multi_ed25519::new_signature_from_bytes(arg1);
            let v8 = 0x1::multi_ed25519::signature_verify_strict_t<RotationCapabilityOfferProofChallengeV2>(&v7, &v6, v3);
            assert!(v8, 0x1::error::invalid_argument(8));
        };
        0x1::option::swap_or_fill<address>(&mut v1.rotation_capability_offer.for, arg4);
    }
    
    public entry fun offer_signer_capability(arg0: &signer, arg1: vector<u8>, arg2: u8, arg3: vector<u8>, arg4: address) acquires Account {
        let v0 = 0x1::signer::address_of(arg0);
        assert!(exists_at(arg4), 0x1::error::not_found(2));
        let v1 = get_sequence_number(v0);
        let v2 = SignerCapabilityOfferProofChallengeV2{
            sequence_number   : v1, 
            source_address    : v0, 
            recipient_address : arg4,
        };
        verify_signed_message<SignerCapabilityOfferProofChallengeV2>(v0, arg2, arg3, arg1, v2);
        let v3 = &mut borrow_global_mut<Account>(v0).signer_capability_offer.for;
        0x1::option::swap_or_fill<address>(v3, arg4);
    }
    
    public(friend) fun register_coin<T0>(arg0: address) acquires Account {
        let v0 = &mut borrow_global_mut<Account>(arg0).coin_register_events;
        let v1 = CoinRegisterEvent{type_info: 0x1::type_info::type_of<T0>()};
        0x1::event::emit_event<CoinRegisterEvent>(v0, v1);
    }
    
    public entry fun revoke_any_rotation_capability(arg0: &signer) acquires Account {
        let v0 = &mut borrow_global_mut<Account>(0x1::signer::address_of(arg0)).rotation_capability_offer.for;
        0x1::option::extract<address>(v0);
    }
    
    public entry fun revoke_any_signer_capability(arg0: &signer) acquires Account {
        let v0 = &mut borrow_global_mut<Account>(0x1::signer::address_of(arg0)).signer_capability_offer.for;
        0x1::option::extract<address>(v0);
    }
    
    public entry fun revoke_rotation_capability(arg0: &signer, arg1: address) acquires Account {
        assert!(exists_at(arg1), 0x1::error::not_found(2));
        let v0 = &borrow_global_mut<Account>(0x1::signer::address_of(arg0)).rotation_capability_offer.for;
        assert!(0x1::option::contains<address>(v0, &arg1), 0x1::error::not_found(18));
        revoke_any_rotation_capability(arg0);
    }
    
    public entry fun revoke_signer_capability(arg0: &signer, arg1: address) acquires Account {
        assert!(exists_at(arg1), 0x1::error::not_found(2));
        let v0 = &borrow_global_mut<Account>(0x1::signer::address_of(arg0)).signer_capability_offer.for;
        assert!(0x1::option::contains<address>(v0, &arg1), 0x1::error::not_found(14));
        revoke_any_signer_capability(arg0);
    }
    
    public entry fun rotate_authentication_key(arg0: &signer, arg1: u8, arg2: vector<u8>, arg3: u8, arg4: vector<u8>, arg5: vector<u8>, arg6: vector<u8>) acquires Account, OriginatingAddress {
        let v0 = 0x1::signer::address_of(arg0);
        assert!(exists_at(v0), 0x1::error::not_found(2));
        let v1 = borrow_global_mut<Account>(v0);
        if (arg1 == 0) {
            let v2 = 0x1::ed25519::new_unvalidated_public_key_from_bytes(arg2);
            let v3 = v1.authentication_key == 0x1::ed25519::unvalidated_public_key_to_authentication_key(&v2);
            assert!(v3, 0x1::error::unauthenticated(7));
        } else {
            assert!(arg1 == 1, 0x1::error::invalid_argument(12));
            let v4 = 0x1::multi_ed25519::new_unvalidated_public_key_from_bytes(arg2);
            let v5 = v1.authentication_key == 0x1::multi_ed25519::unvalidated_public_key_to_authentication_key(&v4);
            assert!(v5, 0x1::error::unauthenticated(7));
        };
        let v6 = 0x1::from_bcs::to_address(v1.authentication_key);
        let v7 = v1.sequence_number;
        let v8 = RotationProofChallenge{
            sequence_number  : v7, 
            originator       : v0, 
            current_auth_key : v6, 
            new_public_key   : arg4,
        };
        assert_valid_rotation_proof_signature_and_get_auth_key(arg1, arg2, arg5, &v8);
        let v9 = assert_valid_rotation_proof_signature_and_get_auth_key(arg3, arg4, arg6, &v8);
        update_auth_key_and_originating_address_table(v0, v1, v9);
    }
    
    public(friend) fun rotate_authentication_key_internal(arg0: &signer, arg1: vector<u8>) acquires Account {
        let v0 = 0x1::signer::address_of(arg0);
        assert!(exists_at(v0), 0x1::error::not_found(2));
        assert!(0x1::vector::length<u8>(&arg1) == 32, 0x1::error::invalid_argument(4));
        borrow_global_mut<Account>(v0).authentication_key = arg1;
    }
    
    public entry fun rotate_authentication_key_with_rotation_capability(arg0: &signer, arg1: address, arg2: u8, arg3: vector<u8>, arg4: vector<u8>) acquires Account, OriginatingAddress {
        assert!(exists_at(arg1), 0x1::error::not_found(17));
        let v0 = 0x1::signer::address_of(arg0);
        let v1 = borrow_global<Account>(arg1);
        assert!(0x1::option::contains<address>(&v1.rotation_capability_offer.for, &v0), 0x1::error::not_found(18));
        let v2 = 0x1::from_bcs::to_address(v1.authentication_key);
        let v3 = get_sequence_number(v0);
        let v4 = RotationProofChallenge{
            sequence_number  : v3, 
            originator       : arg1, 
            current_auth_key : v2, 
            new_public_key   : arg3,
        };
        let v5 = assert_valid_rotation_proof_signature_and_get_auth_key(arg2, arg3, arg4, &v4);
        update_auth_key_and_originating_address_table(arg1, borrow_global_mut<Account>(arg1), v5);
    }
    
    fun update_auth_key_and_originating_address_table(arg0: address, arg1: &mut Account, arg2: vector<u8>) acquires OriginatingAddress {
        let v0 = &mut borrow_global_mut<OriginatingAddress>(@0x1).address_map;
        let v1 = 0x1::from_bcs::to_address(arg1.authentication_key);
        if (0x1::table::contains<address, address>(v0, v1)) {
            assert!(arg0 == 0x1::table::remove<address, address>(v0, v1), 0x1::error::not_found(13));
        };
        0x1::table::add<address, address>(v0, 0x1::from_bcs::to_address(arg2), arg0);
        let v2 = arg1.authentication_key;
        let v3 = KeyRotation{
            account                : arg0, 
            old_authentication_key : v2, 
            new_authentication_key : arg2,
        };
        0x1::event::emit<KeyRotation>(v3);
        let v4 = KeyRotationEvent{
            old_authentication_key : arg1.authentication_key, 
            new_authentication_key : arg2,
        };
        0x1::event::emit_event<KeyRotationEvent>(&mut arg1.key_rotation_events, v4);
        arg1.authentication_key = arg2;
    }
    
    public fun verify_signed_message<T0: drop>(arg0: address, arg1: u8, arg2: vector<u8>, arg3: vector<u8>, arg4: T0) acquires Account {
        let v0 = borrow_global_mut<Account>(arg0);
        if (arg1 == 0) {
            let v1 = 0x1::ed25519::new_unvalidated_public_key_from_bytes(arg2);
            let v2 = v0.authentication_key == 0x1::ed25519::unvalidated_public_key_to_authentication_key(&v1);
            assert!(v2, 0x1::error::invalid_argument(7));
            let v3 = 0x1::ed25519::new_signature_from_bytes(arg3);
            let v4 = 0x1::ed25519::signature_verify_strict_t<T0>(&v3, &v1, arg4);
            assert!(v4, 0x1::error::invalid_argument(8));
        } else {
            assert!(arg1 == 1, 0x1::error::invalid_argument(12));
            let v5 = 0x1::multi_ed25519::new_unvalidated_public_key_from_bytes(arg2);
            let v6 = v0.authentication_key == 0x1::multi_ed25519::unvalidated_public_key_to_authentication_key(&v5);
            assert!(v6, 0x1::error::invalid_argument(7));
            let v7 = 0x1::multi_ed25519::new_signature_from_bytes(arg3);
            let v8 = 0x1::multi_ed25519::signature_verify_strict_t<T0>(&v7, &v5, arg4);
            assert!(v8, 0x1::error::invalid_argument(8));
        };
    }
    
    // decompiled from Move bytecode v6
}
