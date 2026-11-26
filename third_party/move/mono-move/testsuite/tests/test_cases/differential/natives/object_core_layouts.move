// `Object<T>` and `ObjectCore` from a stand-in `0x1::object` module with the framework's
// names (the event handle of `ObjectCore` is a vector here, so both resources hold a vector,
// in different field positions). Steps: wrap an address in an `Object<Res>` and read it
// back; create `ObjectCore`, `Res` and a resource holding an `Object<Res>`; modify both
// resources through the stored `Object<Res>`, forcing collections while they are borrowed;
// read everything back.

// RUN: publish
module 0x1::object {
    struct ObjectCore has key {
        guid_creation_num: u64,
        owner: address,
        allow_ungated_transfer: bool,
        transfer_events: vector<u64>,
    }

    struct Object<phantom T> has copy, drop, store {
        inner: address,
    }

    native fun exists_at<T: key>(object: address): bool;

    // Packs without the existence checks of `address_to_object`.
    public fun wrap<T: key>(object: address): Object<T> {
        Object<T> { inner: object }
    }

    public fun address_to_object<T: key>(object: address): Object<T> {
        assert!(exists<ObjectCore>(object), 1);
        assert!(exists_at<T>(object), 2);
        Object<T> { inner: object }
    }

    public fun object_address<T: key>(object: &Object<T>): address {
        object.inner
    }

    public fun init_core(object: &signer, owner: address) {
        move_to(object, ObjectCore {
            guid_creation_num: 0x1122334455667788,
            owner,
            allow_ungated_transfer: true,
            transfer_events: vector[7],
        });
    }

    public fun record_transfer(object: address, event: u64): (u64, address, bool) acquires ObjectCore {
        let core = borrow_global_mut<ObjectCore>(object);
        core.transfer_events.push_back(event);
        0x0::test_utils::force_gc();
        (core.transfer_events.length(), core.owner, core.allow_ungated_transfer)
    }

    public fun events(object: address): vector<u64> acquires ObjectCore {
        borrow_global<ObjectCore>(object).transfer_events
    }
}

module 0x42::user {
    use 0x1::object::{Self, Object};

    struct Res has key { tag: u8, data: vector<u64>, owner: address }

    struct Pointer has key { target: Object<Res>, label: u16 }

    public fun object_of(addr: address): address {
        let obj = object::wrap<Res>(addr);
        object::object_address(&obj)
    }

    public fun create(obj_signer: signer, holder: signer, owner: address) {
        let addr = std::signer::address_of(&obj_signer);
        object::init_core(&obj_signer, owner);
        move_to(&obj_signer, Res { tag: 0x5a, data: vector[1, 2], owner });
        move_to(&holder, Pointer { target: object::address_to_object<Res>(addr), label: 0xbeef });
    }

    public fun update(holder: address): (u64, address, bool, u64, u64, u16) acquires Res, Pointer {
        let pointer = borrow_global<Pointer>(holder);
        let addr = object::object_address(&pointer.target);
        let res = borrow_global_mut<Res>(addr);
        res.data.push_back(300);
        0x0::test_utils::force_gc();
        let (count, core_owner, ungated) = object::record_transfer(addr, 400);
        0x0::test_utils::force_gc();
        let res = borrow_global<Res>(addr);
        (count, core_owner, ungated, res.data[0] + res.data[1] + res.data[2], (res.tag as u64), pointer.label)
    }

    public fun read(holder: address): (vector<u64>, vector<u64>, address) acquires Res, Pointer {
        let obj = borrow_global<Pointer>(holder).target;
        let addr = object::object_address(&object::address_to_object<Res>(object::object_address(&obj)));
        (borrow_global<Res>(addr).data, object::events(addr), borrow_global<Res>(addr).owner)
    }
}

// RUN: execute 0x42::user::object_of --args 0xab
// CHECK: results: 0xab

// RUN: execute 0x42::user::create --args 0xab, 0x42, 0xc0ffee00000000000000000000000000000000000000000000000000000000ee --commit
// CHECK: writes: create 0x42::user::Pointer at 0x42 0x00000000000000000000000000000000000000000000000000000000000000abefbe; create 0x1::object::ObjectCore at 0xab 0x8877665544332211c0ffee00000000000000000000000000000000000000000000000000000000ee01010700000000000000; create 0x42::user::Res at 0xab 0x5a0201000000000000000200000000000000c0ffee00000000000000000000000000000000000000000000000000000000ee

// RUN: execute 0x42::user::update --args 0x42 --commit
// CHECK: results: 2, 0xc0ffee00000000000000000000000000000000000000000000000000000000ee, true, 303, 90, 48879 | writes: modify 0x1::object::ObjectCore at 0xab 0x8877665544332211c0ffee00000000000000000000000000000000000000000000000000000000ee010207000000000000009001000000000000; modify 0x42::user::Res at 0xab 0x5a03010000000000000002000000000000002c01000000000000c0ffee00000000000000000000000000000000000000000000000000000000ee
// CHECK-GC-COUNT: 3

// RUN: execute 0x42::user::read --args 0x42
// CHECK: results: [1, 2, 300], [7, 400], 0xc0ffee00000000000000000000000000000000000000000000000000000000ee
