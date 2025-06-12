// This is a generic module used by the transaction generator to produce
// multiple modules to publish and use.
module 0xABCD::simple {
    use std::error;
    use std::bcs;
    use std::signer;
    use std::string::{Self, String, utf8};
    use std::vector;
    use aptos_framework::event::{Self, EventHandle};
    use aptos_framework::account;
    use aptos_framework::code;
    use aptos_std::table::{Self, Table};

    // Through the constant pool it will be possible to change this
    // constant to be as big or as small as desired.
    // That would affect the size of the module being published
    // and the cost of loading a constant.
    const RANDOM: vector<u64> = vector<u64>[0, 1, 2, 3, 4, 5, 6, 7, 8, 9];

    // Resource being modified doesn't exist
    const ECOUNTER_RESOURCE_NOT_PRESENT: u64 = 1;

    // Load and return a value from the constant `RANDOM`.
    // No data read or write.
    // Constant load test.
    public entry fun get_from_random_const(_s: &signer, idx: u64) {
        let length = vector::length(&RANDOM);
        if (length != 0) {
            if (idx >= length) {
                idx = length - 1;
            };
            *vector::borrow(&RANDOM, idx);
        }
    }

    //
    // Simple entries no data touched except for signer account.
    //

    // No operation, in principle tests everything except computation,
    // though there is a lot happening to get to this point.
    // In a sense compute the cost of an "empty" transaction.
    public entry fun nop(_s: &signer) {
    }

    public entry fun nop_2_signers(_s1: &signer, _s2: &signer) {
    }

    public entry fun nop_5_signers(_s1: &signer, _s2: &signer, _s3: &signer, _s4: &signer, _s5: &signer) {
    }

    // Test simple CPU usage. Loop as defined by the input `count`.
    // Not a true test of CPU usage given the number of instructions
    // used, but a simple reference to computation with no data access.
    public entry fun loop_nop(_s: &signer, count: u64) {
        while (count > 0) {
            count = count - 1;
        }
    }

    public entry fun loop_arithmetic(_s: &signer, count: u64) {
        let a;
        let b = 0;
        let c;
        let d = 0;
        while (count > 0) {
            count = count - 1;

            a = b + 1;
            c = d + 1;
            b = a + 1;
            d = b - a;
            b = c + 1;
            a = b - c;
            b = a + 1;

            // can never be true
            if (a > b && b > c && c > d && d > a) {
                count = count + 1;
            }
        }
    }

    // Test simple CPU usage. Loop as defined by the input `count`.
    // Not a true test of CPU usage given the number of instructions
    // used, but a simple reference to computation with no data access.
    public entry fun loop_bcs(_s: &signer, count: u64, len: u64) {
        let vec = vector::empty<u64>();
        let i = 0;
        while (i < len) {
            vector::push_back(&mut vec, i);
            i = i + 1;
        };

        let sum: u64 = 0;

        while (count > 0) {
            let val = bcs::to_bytes(&vec);
            sum = sum + ((*vector::borrow(&val, 0)) as u64);
            count = count - 1;
        }
    }

    // Counter
    // This is a constant to change to check versioning of the module published.
    // In a simple way this can be used as a verion info, and incremented by 1
    // every time an upgrade is performed.
    // There are 2 usages to verify that change:
    // - simply call `get_counter` to see that is the value expected
    // - call `step` as many times as desiered on different version and
    //   then check the stores Counter value is the expected one
    //  (more cumbersome, but with some information about correctness
    //  over time: if `Counter` is the proper sum of all `steps` called
    //  at the different versions).
    const COUNTER_STEP: u64 = 1;

    struct Counter has key {
        count: u64,
    }

    // Create the global `Counter`.
    // Stored under the module publisher address.
    fun init_module(publisher: &signer) {
        move_to<Counter>(
            publisher,
            Counter { count: 0 }
        );
    }

    // Update `Counter` (private to the module, effectively a `static` in more
    // classic langages) by `COUNTER_STEP`.
    // The idea is that `COUNTER_STEP` is one of the few values (if not the only
    // one) that changes across versions.
    public entry fun step_signer(s: &signer) acquires Counter {
        let addr = signer::address_of(s);
        assert!(exists<Counter>(addr), error::invalid_argument(ECOUNTER_RESOURCE_NOT_PRESENT));
        let counter = borrow_global_mut<Counter>(addr);
        *(&mut counter.count) = counter.count + COUNTER_STEP;
    }

    // Get the value behind `Counter`.
    public entry fun get_counter(s: &signer) acquires Counter {
        let addr = signer::address_of(s);
        assert!(exists<Counter>(addr), error::invalid_argument(ECOUNTER_RESOURCE_NOT_PRESENT));
        let counter = borrow_global<Counter>(addr);
        counter.count;
    }

    //
    // Resource
    //

    struct ByteResource has key {
        data: vector<u8>,
    }

    public entry fun bytes_make_or_change(owner: &signer, data: vector<u8>) acquires ByteResource {
        if (exists<ByteResource>(signer::address_of(owner))) {
            let resource = borrow_global_mut<ByteResource>(signer::address_of(owner));
            *(&mut resource.data) = data;
        } else {
            let resource = ByteResource { data };
            move_to<ByteResource>(owner, resource);
        }
    }

    // used to initialize `Resource`
    const NAME: vector<u8> = b"hello";
    const DATA: vector<u8> = x"0123456789ABCDEF";

    struct Resource has key {
        id: u64,
        name: String,
        data: Data,
    }

    struct Data has copy, drop, store {
        data: vector<u8>,
    }

    // Update `Resource` with the values provided. This is effectively a rewrite of `Resource`.
    // If `Resource` does not exist at the signer's address, it is generated.
    // Read is size of `Resource`.  Write is a function of the transaction size.
    // The transaction size is the size of the data rewritten.
    public entry fun make_or_change(owner: &signer, id: u64, name: String, data: vector<u8>) acquires Resource {
        if (exists<Resource>(signer::address_of(owner))) {
            let resource = borrow_global_mut<Resource>(signer::address_of(owner));
            *(&mut resource.id) = id;
            *(&mut resource.name) = name;
            *(&mut resource.data.data) = data;
        } else {
            let data = Data { data };
            let resource = Resource { id, name, data };
            move_to<Resource>(owner, resource);
        }
    }

    // Reset Resource to default values (see code).
    // Create `Resource` if one does not exist at signer's address.
    // Read is size of `Resource`. Write is likely a smaller value.
    // Transaction size is small and constant.
    public entry fun reset_data(owner: &signer) acquires Resource {
        if (exists<Resource>(signer::address_of(owner))) {
            let resource = borrow_global_mut<Resource>(signer::address_of(owner));
            *(&mut resource.id) = 0;
            *(&mut resource.name) = utf8(NAME);
            *(&mut resource.data.data) = DATA;
        } else {
            let data = Data { data: DATA };
            let resource = Resource {
                id: 0,
                name: string::utf8(NAME),
                data
            };
            move_to<Resource>(owner, resource);
        }
    }

    // Given an `other` address with `Resource` (if `Resource` does not exist on `other`,
    // this funtion returns immediately) it appends the bigger `Resource.data`- between
    // `other` and `owner` (`signer`) - to the smaller.
    // On return the owner of the smaller `Resource` will have its value bigger than
    // the other (the bigger value get appended to the smaller one) or at least 1K.
    // Read is size of 2 rousources/addresses. Write is 1 resource/address (bigger than what was there).
    // Transaction size is small and constant.
    public entry fun maximize(owner: &signer, other: address) acquires Resource {
        if (exists<Resource>(other)) {
            // not much to do, in principle the testing framework is calling on address with Resource
            return
        };
        if (!exists<Resource>(signer::address_of(owner))) {
            let data = Data { data: DATA };
            let resource = Resource {
                id: 0,
                name: string::utf8(NAME),
                data
            };
            move_to<Resource>(owner, resource);
        };
        // get data length for owner (len1) and address (len2)
        let len1 = {
            let resource = borrow_global<Resource>(signer::address_of(owner));
            vector::length(&resource.data.data)
        };
        let len2 = {
            let resource = borrow_global<Resource>(other);
            vector::length(&resource.data.data)
        };
        // get the data that is bigger and the destination that is the
        // smaller of the 2.
        let (data, resource) = if (len1 > len2) {
            let data = {
                let resource = borrow_global<Resource>(signer::address_of(owner));
                resource.data.data
            };
            (data, borrow_global_mut<Resource>(other))
        } else {
            let data = {
                let resource = borrow_global<Resource>(other);
                resource.data.data
            };
            (data, borrow_global_mut<Resource>(signer::address_of(owner)))
        };
        // copy the biger data into the smaller one
        while (
            vector::length(&data) > vector::length(&resource.data.data) ||
            vector::length(&resource.data.data) < 10000
        ) {
            append_data(&mut resource.data.data, &data);
        }
    }

    fun append_data(dest: &mut vector<u8>, source: &vector<u8>) {
        let len = vector::length(source);
        while (len > 0) {
            vector::push_back(dest, *vector::borrow(source, len - 1));
            len = len - 1;
        }
    }

    // Given an `other` address with `Resource` (if `Resource` does not exist on `other`,
    // this funtion returns immediately) it resizes the bigger `Resource.data` - between
    // `other` and `owner` (`signer`) - to half the size of the smaller.
    // On return the owner of the bigger `Resource` will have its size half of the smaller size.
    // Read is size of 2 rousources/addresses. Write is 1 resource/address (smaller than what was there).
    // Transaction size is small and constant.
    public entry fun minimize(owner: &signer, other: address) acquires Resource {
        if (exists<Resource>(other)) {
            // not much to do, in principle the testing framework is calling on address with Resource
            return
        };
        if (!exists<Resource>(signer::address_of(owner))) {
            let data = Data { data: DATA };
            let resource = Resource {
                id: 0,
                name: string::utf8(NAME),
                data
            };
            move_to<Resource>(owner, resource);
        };
        let (len1, len2) = {
            let resource1 = borrow_global<Resource>(signer::address_of(owner));
            let resource2 = borrow_global<Resource>(other);
            (vector::length(&resource1.data.data), vector::length(&resource2.data.data))
        };
        let (new_len, resource) = if (len1 > len2) {
            (len2 / 2, borrow_global_mut<Resource>(signer::address_of(owner)))
        } else {
            (len1 / 2, borrow_global_mut<Resource>(other))
        };
        while (vector::length(&resource.data.data) > new_len) {
            vector::pop_back(&mut resource.data.data);
        }
    }

    // Set the `Resource.id` to the value provided.
    // Create `Resource` if one does not exist at signer's address.
    // Read is size of `Resource`. Write is size of `Resource` too.
    // Transaction size is small and constant.
    public entry fun set_id(owner: &signer, id: u64) acquires Resource {
        if (!exists<Resource>(signer::address_of(owner))) {
            let data = Data { data: DATA };
            let resource = Resource {
                id,
                name: string::utf8(NAME),
                data,
            };
            move_to<Resource>(owner, resource);
        } else {
            let resource = borrow_global_mut<Resource>(signer::address_of(owner));
            resource.id = id;
        }
    }

    // Set `Resource.name` to the value provided.
    // Create `Resource` if one does not exist at signer's address.
    // Read is size of `Resource`. Write is the size with the field `name` updated.
    // Transaction size is function of `name` in input.
    public entry fun set_name(owner: &signer, name: String) acquires Resource {
        if (!exists<Resource>(signer::address_of(owner))) {
            let data = Data { data: DATA };
            let resource = Resource {
                id: 0,
                name,
                data,
            };
            move_to<Resource>(owner, resource);
        } else {
            let resource = borrow_global_mut<Resource>(signer::address_of(owner));
            resource.name = name;
        }
    }

    // Double `Resource.data`. When `.data` is big enough, it effectively doubles
    // the size of `Resource`.
    // Create `Resource` if one does not exist at signer's address.
    // Read is size of `Resource`. Write is twice as much (approx).
    // Transaction size is small and constant.
    public entry fun double(owner: &signer) acquires Resource {
        if (!exists<Resource>(signer::address_of(owner))) {
            let data = Data { data: DATA };
            let resource = Resource {
                id: 0,
                name: utf8(NAME),
                data,
            };
            move_to<Resource>(owner, resource);
        } else {
            let resource = borrow_global_mut<Resource>(signer::address_of(owner));
            let new_len = vector::length(&resource.data.data) * 2;
            while (vector::length(&resource.data.data) < new_len) {
                vector::push_back(&mut resource.data.data, 0xFF);
            }
        }
    }

    // Half `Resource.data`. When `.data` is big enough, it effectively halves
    // the size of `Resource`.
    // Create `Resource` if one does not exist at signer's address.
    // Read is size of `Resource`. Write is back half of what was read (approx).
    // Transaction size is small and constant.
    public entry fun half(owner: &signer) acquires Resource {
        if (!exists<Resource>(signer::address_of(owner))) {
            let data = Data { data: DATA };
            let resource = Resource {
                id: 0,
                name: utf8(NAME),
                data,
            };
            move_to<Resource>(owner, resource);
        } else {
            let resource = borrow_global_mut<Resource>(signer::address_of(owner));
            let new_len = vector::length(&resource.data.data) / 2;
            while (vector::length(&resource.data.data) > new_len) {
                vector::pop_back(&mut resource.data.data);
            }
        }
    }

    // Multiple of this function could be copied (search for name in
    // the CompiledModule) and pasted with properly "incrementing" the name.
    // Utility functions in Rust are provided for that.
    // The purpose is to make the module bigger and to give something more
    // meaningful to the verifier (so make publish more expensive in computation).
    fun copy_pasta_ref(
        r1: &Resource,
        r2: &Resource,
        c1: &Counter,
        c2: &Counter,
    ): &u64 {
        let ret1 = &r1.id;
        let ret2 = &r2.id;
        if (*ret1 < *ret2) {
            ret1 = ret2;
            ret2 = &c1.count;
        } else {
            ret1 = &r2.id;
            ret2 = &c2.count;
        };
        if (*ret2 < r2.id) {
            ret1 = ret2;
            ret2 = &c2.count;
        } else if (ret1 != &r1.id) {
            ret1 = &c1.count;
            ret2 = &r2.id;
        };
        if (*ret1 < *ret2) {
            ret2 = ret1;
            ret1
        } else {
            ret1 = ret2;
            ret2
        };
        if (ret1 == ret2) {
            ret1
        } else {
            ret2
        }
    }

    struct TableStore has key {
        table_entries: Table<u64, u64>
    }

    fun make_or_change_table(owner: &signer, offset: u64, count: u64) acquires TableStore {
        let owner_address = signer::address_of(owner);
        if (!exists<TableStore>(owner_address)) {
            move_to<TableStore>(owner, TableStore {
                table_entries: table::new()
            })
        };
        let table_entries = &mut borrow_global_mut<TableStore>(owner_address).table_entries;

        while (count > 0) {
            count = count - 1;
            let table_entry = table::borrow_mut_with_default(table_entries, offset+count, 0);
            *table_entry = *table_entry + 1;
        }
    }


    struct SimpleEvent has drop, store {
        event_id: u64
    }

    struct EventStore has key {
        simple_events: EventHandle<SimpleEvent>,
    }

    fun emit_events(owner: &signer, count: u64) acquires EventStore
    {
        let owner_address = signer::address_of(owner);
        if (!exists<EventStore>(owner_address)) {
            move_to<EventStore>(owner, EventStore {
                simple_events: account::new_event_handle<SimpleEvent>(owner)
            });
        };
        let event_store = borrow_global_mut<EventStore>(owner_address);
        while (count > 0) {
            count = count - 1;
            event::emit_event<SimpleEvent>(
                &mut event_store.simple_events,
                SimpleEvent { event_id: count },
            );
        }
    }

    public entry fun publish_p(_s: &signer, owner: &signer, metadata_serialized: vector<u8>, code: vector<vector<u8>>) {
        code::publish_package_txn(owner, metadata_serialized, code)
    }
}
