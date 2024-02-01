module 0x1::object {
    struct ConstructorRef has drop {
        self: address,
        can_delete: bool,
    }
    
    struct DeleteRef has drop, store {
        self: address,
    }
    
    struct DeriveRef has drop, store {
        self: address,
    }
    
    struct ExtendRef has drop, store {
        self: address,
    }
    
    struct LinearTransferRef has drop {
        self: address,
        owner: address,
    }
    
    struct Object<phantom T0> has copy, drop, store {
        inner: address,
    }
    
    struct ObjectCore has key {
        guid_creation_num: u64,
        owner: address,
        allow_ungated_transfer: bool,
        transfer_events: 0x1::event::EventHandle<TransferEvent>,
    }
    
    struct ObjectGroup {
        dummy_field: bool,
    }
    
    struct TombStone has key {
        original_owner: address,
    }
    
    struct TransferEvent has drop, store {
        object: address,
        from: address,
        to: address,
    }
    
    struct TransferRef has drop, store {
        self: address,
    }
    
    public fun create_guid(arg0: &signer) : 0x1::guid::GUID acquires ObjectCore {
        let v0 = 0x1::signer::address_of(arg0);
        0x1::guid::create(v0, &mut borrow_global_mut<ObjectCore>(v0).guid_creation_num)
    }
    
    public fun new_event_handle<T0: drop + store>(arg0: &signer) : 0x1::event::EventHandle<T0> acquires ObjectCore {
        let v0 = create_guid(arg0);
        0x1::event::new_event_handle<T0>(v0)
    }
    
    public fun address_from_constructor_ref(arg0: &ConstructorRef) : address {
        arg0.self
    }
    
    public fun address_from_delete_ref(arg0: &DeleteRef) : address {
        arg0.self
    }
    
    public fun address_from_extend_ref(arg0: &ExtendRef) : address {
        arg0.self
    }
    
    public fun address_to_object<T0: key>(arg0: address) : Object<T0> {
        assert!(exists<ObjectCore>(arg0), 0x1::error::not_found(2));
        assert!(exists_at<T0>(arg0), 0x1::error::not_found(7));
        Object<T0>{inner: arg0}
    }
    
    public entry fun burn<T0: key>(arg0: &signer, arg1: Object<T0>) acquires ObjectCore {
        let v0 = 0x1::signer::address_of(arg0);
        assert!(is_owner<T0>(arg1, v0), 0x1::error::permission_denied(4));
        let v1 = arg1.inner;
        let v2 = 0x1::create_signer::create_signer(v1);
        let v3 = TombStone{original_owner: v0};
        move_to<TombStone>(&v2, v3);
        let v4 = @0xffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff;
        let v5 = borrow_global_mut<ObjectCore>(v1);
        if (v5.owner != v4) {
            let v6 = TransferEvent{
                object : v1, 
                from   : v5.owner, 
                to     : v4,
            };
            0x1::event::emit<TransferEvent>(v6);
            let v7 = TransferEvent{
                object : v1, 
                from   : v5.owner, 
                to     : v4,
            };
            0x1::event::emit_event<TransferEvent>(&mut v5.transfer_events, v7);
            v5.owner = v4;
        };
    }
    
    public fun can_generate_delete_ref(arg0: &ConstructorRef) : bool {
        arg0.can_delete
    }
    
    public fun convert<T0: key, T1: key>(arg0: Object<T0>) : Object<T1> {
        address_to_object<T1>(arg0.inner)
    }
    
    public fun create_guid_object_address(arg0: address, arg1: u64) : address {
        let v0 = 0x1::guid::create_id(arg0, arg1);
        let v1 = 0x1::bcs::to_bytes<0x1::guid::ID>(&v0);
        0x1::vector::push_back<u8>(&mut v1, 253);
        0x1::from_bcs::to_address(0x1::hash::sha3_256(v1))
    }
    
    public fun create_named_object(arg0: &signer, arg1: vector<u8>) : ConstructorRef {
        let v0 = 0x1::signer::address_of(arg0);
        create_object_internal(v0, create_object_address(&v0, arg1), false)
    }
    
    public fun create_object(arg0: address) : ConstructorRef {
        create_object_internal(arg0, 0x1::transaction_context::generate_auid_address(), true)
    }
    
    public fun create_object_address(arg0: &address, arg1: vector<u8>) : address {
        let v0 = 0x1::bcs::to_bytes<address>(arg0);
        0x1::vector::append<u8>(&mut v0, arg1);
        0x1::vector::push_back<u8>(&mut v0, 254);
        0x1::from_bcs::to_address(0x1::hash::sha3_256(v0))
    }
    
    public fun create_object_from_account(arg0: &signer) : ConstructorRef {
        create_object_from_guid(0x1::signer::address_of(arg0), 0x1::account::create_guid(arg0))
    }
    
    fun create_object_from_guid(arg0: address, arg1: 0x1::guid::GUID) : ConstructorRef {
        let v0 = 0x1::bcs::to_bytes<0x1::guid::GUID>(&arg1);
        0x1::vector::push_back<u8>(&mut v0, 253);
        create_object_internal(arg0, 0x1::from_bcs::to_address(0x1::hash::sha3_256(v0)), true)
    }
    
    public fun create_object_from_object(arg0: &signer) : ConstructorRef acquires ObjectCore {
        let v0 = create_guid(arg0);
        create_object_from_guid(0x1::signer::address_of(arg0), v0)
    }
    
    fun create_object_internal(arg0: address, arg1: address, arg2: bool) : ConstructorRef {
        assert!(!exists<ObjectCore>(arg1), 0x1::error::already_exists(1));
        let v0 = 0x1::create_signer::create_signer(arg1);
        let v1 = 1125899906842624;
        let v2 = 0x1::event::new_event_handle<TransferEvent>(0x1::guid::create(arg1, &mut v1));
        let v3 = ObjectCore{
            guid_creation_num      : v1, 
            owner                  : arg0, 
            allow_ungated_transfer : true, 
            transfer_events        : v2,
        };
        move_to<ObjectCore>(&v0, v3);
        ConstructorRef{
            self       : arg1, 
            can_delete : arg2,
        }
    }
    
    public fun create_sticky_object(arg0: address) : ConstructorRef {
        create_object_internal(arg0, 0x1::transaction_context::generate_auid_address(), false)
    }
    
    public(friend) fun create_user_derived_object(arg0: address, arg1: &DeriveRef) : ConstructorRef {
        create_object_internal(arg0, create_user_derived_object_address(arg0, arg1.self), false)
    }
    
    public fun create_user_derived_object_address(arg0: address, arg1: address) : address {
        let v0 = 0x1::bcs::to_bytes<address>(&arg0);
        0x1::vector::append<u8>(&mut v0, 0x1::bcs::to_bytes<address>(&arg1));
        0x1::vector::push_back<u8>(&mut v0, 252);
        0x1::from_bcs::to_address(0x1::hash::sha3_256(v0))
    }
    
    public fun delete(arg0: DeleteRef) acquires ObjectCore {
        let ObjectCore {
            guid_creation_num      : _,
            owner                  : _,
            allow_ungated_transfer : _,
            transfer_events        : v3,
        } = move_from<ObjectCore>(arg0.self);
        0x1::event::destroy_handle<TransferEvent>(v3);
    }
    
    public fun disable_ungated_transfer(arg0: &TransferRef) acquires ObjectCore {
        borrow_global_mut<ObjectCore>(arg0.self).allow_ungated_transfer = false;
    }
    
    public fun enable_ungated_transfer(arg0: &TransferRef) acquires ObjectCore {
        borrow_global_mut<ObjectCore>(arg0.self).allow_ungated_transfer = true;
    }
    
    native fun exists_at<T0: key>(arg0: address) : bool;
    public fun generate_delete_ref(arg0: &ConstructorRef) : DeleteRef {
        assert!(arg0.can_delete, 0x1::error::permission_denied(5));
        DeleteRef{self: arg0.self}
    }
    
    public fun generate_derive_ref(arg0: &ConstructorRef) : DeriveRef {
        DeriveRef{self: arg0.self}
    }
    
    public fun generate_extend_ref(arg0: &ConstructorRef) : ExtendRef {
        ExtendRef{self: arg0.self}
    }
    
    public fun generate_linear_transfer_ref(arg0: &TransferRef) : LinearTransferRef acquires ObjectCore {
        let v0 = Object<ObjectCore>{inner: arg0.self};
        let v1 = owner<ObjectCore>(v0);
        LinearTransferRef{
            self  : arg0.self, 
            owner : v1,
        }
    }
    
    public fun generate_signer(arg0: &ConstructorRef) : signer {
        0x1::create_signer::create_signer(arg0.self)
    }
    
    public fun generate_signer_for_extending(arg0: &ExtendRef) : signer {
        0x1::create_signer::create_signer(arg0.self)
    }
    
    public fun generate_transfer_ref(arg0: &ConstructorRef) : TransferRef {
        TransferRef{self: arg0.self}
    }
    
    public fun is_burnt<T0: key>(arg0: Object<T0>) : bool {
        exists<TombStone>(arg0.inner)
    }
    
    public fun is_object(arg0: address) : bool {
        exists<ObjectCore>(arg0)
    }
    
    public fun is_owner<T0: key>(arg0: Object<T0>, arg1: address) : bool acquires ObjectCore {
        let v0 = owner<T0>(arg0);
        v0 == arg1
    }
    
    public fun object_address<T0: key>(arg0: &Object<T0>) : address {
        arg0.inner
    }
    
    public fun object_exists<T0: key>(arg0: address) : bool {
        exists<ObjectCore>(arg0) && exists_at<T0>(arg0)
    }
    
    public fun object_from_constructor_ref<T0: key>(arg0: &ConstructorRef) : Object<T0> {
        address_to_object<T0>(arg0.self)
    }
    
    public fun object_from_delete_ref<T0: key>(arg0: &DeleteRef) : Object<T0> {
        address_to_object<T0>(arg0.self)
    }
    
    public fun owner<T0: key>(arg0: Object<T0>) : address acquires ObjectCore {
        assert!(exists<ObjectCore>(arg0.inner), 0x1::error::not_found(2));
        borrow_global<ObjectCore>(arg0.inner).owner
    }
    
    public fun owns<T0: key>(arg0: Object<T0>, arg1: address) : bool acquires ObjectCore {
        let v0 = object_address<T0>(&arg0);
        if (v0 == arg1) {
            return true
        };
        assert!(exists<ObjectCore>(v0), 0x1::error::not_found(2));
        let v1 = borrow_global<ObjectCore>(v0).owner;
        while (arg1 != v1) {
            assert!(0 + 1 < 8, 0x1::error::out_of_range(6));
            if (!exists<ObjectCore>(v1)) {
                return false
            };
            let v2 = &borrow_global<ObjectCore>(v1).owner;
            v1 = *v2;
        };
        true
    }
    
    public entry fun transfer<T0: key>(arg0: &signer, arg1: Object<T0>, arg2: address) acquires ObjectCore {
        transfer_raw(arg0, arg1.inner, arg2);
    }
    
    public entry fun transfer_call(arg0: &signer, arg1: address, arg2: address) acquires ObjectCore {
        transfer_raw(arg0, arg1, arg2);
    }
    
    public fun transfer_raw(arg0: &signer, arg1: address, arg2: address) acquires ObjectCore {
        verify_ungated_and_descendant(0x1::signer::address_of(arg0), arg1);
        let v0 = borrow_global_mut<ObjectCore>(arg1);
        if (v0.owner != arg2) {
            let v1 = TransferEvent{
                object : arg1, 
                from   : v0.owner, 
                to     : arg2,
            };
            0x1::event::emit<TransferEvent>(v1);
            let v2 = TransferEvent{
                object : arg1, 
                from   : v0.owner, 
                to     : arg2,
            };
            0x1::event::emit_event<TransferEvent>(&mut v0.transfer_events, v2);
            v0.owner = arg2;
        };
    }
    
    public entry fun transfer_to_object<T0: key, T1: key>(arg0: &signer, arg1: Object<T0>, arg2: Object<T1>) acquires ObjectCore {
        transfer<T0>(arg0, arg1, arg2.inner);
    }
    
    public fun transfer_with_ref(arg0: LinearTransferRef, arg1: address) acquires ObjectCore {
        let v0 = borrow_global_mut<ObjectCore>(arg0.self);
        assert!(v0.owner == arg0.owner, 0x1::error::permission_denied(4));
        let v1 = TransferEvent{
            object : arg0.self, 
            from   : v0.owner, 
            to     : arg1,
        };
        0x1::event::emit<TransferEvent>(v1);
        let v2 = TransferEvent{
            object : arg0.self, 
            from   : v0.owner, 
            to     : arg1,
        };
        0x1::event::emit_event<TransferEvent>(&mut v0.transfer_events, v2);
        v0.owner = arg1;
    }
    
    public entry fun unburn<T0: key>(arg0: &signer, arg1: Object<T0>) acquires ObjectCore, TombStone {
        let v0 = arg1.inner;
        assert!(exists<TombStone>(v0), 0x1::error::invalid_argument(8));
        let TombStone { original_owner: v1 } = move_from<TombStone>(v0);
        assert!(v1 == 0x1::signer::address_of(arg0), 0x1::error::permission_denied(4));
        let v2 = borrow_global_mut<ObjectCore>(v0);
        if (v2.owner != v1) {
            let v3 = TransferEvent{
                object : v0, 
                from   : v2.owner, 
                to     : v1,
            };
            0x1::event::emit<TransferEvent>(v3);
            let v4 = TransferEvent{
                object : v0, 
                from   : v2.owner, 
                to     : v1,
            };
            0x1::event::emit_event<TransferEvent>(&mut v2.transfer_events, v4);
            v2.owner = v1;
        };
    }
    
    public fun ungated_transfer_allowed<T0: key>(arg0: Object<T0>) : bool acquires ObjectCore {
        assert!(exists<ObjectCore>(arg0.inner), 0x1::error::not_found(2));
        borrow_global<ObjectCore>(arg0.inner).allow_ungated_transfer
    }
    
    fun verify_ungated_and_descendant(arg0: address, arg1: address) acquires ObjectCore {
        assert!(exists<ObjectCore>(arg1), 0x1::error::not_found(2));
        let v0 = borrow_global<ObjectCore>(arg1);
        assert!(v0.allow_ungated_transfer, 0x1::error::permission_denied(3));
        let v1 = v0.owner;
        while (arg0 != v1) {
            assert!(0 + 1 < 8, 0x1::error::out_of_range(6));
            assert!(exists<ObjectCore>(v1), 0x1::error::permission_denied(4));
            let v2 = borrow_global<ObjectCore>(v1);
            assert!(v2.allow_ungated_transfer, 0x1::error::permission_denied(3));
            v1 = v2.owner;
        };
    }
    
    // decompiled from Move bytecode v6
}
