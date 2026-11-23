// Resources keyed by a function type with reference parameters: created, read back in a
// later step, modified and deleted, with the writes pinned. `|&u8|u8` and `|&mut u8|u8` are
// distinct keys at one address, and a struct parameter type is printed with its full path.

// RUN: publish
module 0x42::keys {
    struct Inner has drop { count: u64 }
    struct Tag<phantom T> has key, drop { count: u64 }

    public fun put(owner: signer) {
        move_to(&owner, Tag<|&u8|u8 has copy + drop> { count: 1 });
        move_to(&owner, Tag<|&mut u8|u8 has copy + drop> { count: 2 });
        move_to(&owner, Tag<|&Inner, &vector<u64>|(u64, bool) has copy + drop + store> { count: 3 });
    }

    public fun read(addr: address): u64 {
        borrow_global<Tag<|&u8|u8 has copy + drop>>(addr).count * 100
            + borrow_global<Tag<|&mut u8|u8 has copy + drop>>(addr).count * 10
            + borrow_global<Tag<|&Inner, &vector<u64>|(u64, bool) has copy + drop + store>>(addr).count
    }

    public fun bump(addr: address) {
        borrow_global_mut<Tag<|&u8|u8 has copy + drop>>(addr).count = 0x0102030405060708;
        borrow_global_mut<Tag<|&Inner, &vector<u64>|(u64, bool) has copy + drop + store>>(addr).count = 9;
    }

    public fun take(addr: address): u64 {
        let Tag { count } = move_from<Tag<|&u8|u8 has copy + drop>>(addr);
        let Tag { count: other } = move_from<Tag<|&Inner, &vector<u64>|(u64, bool) has copy + drop + store>>(addr);
        count + other
    }
}

// RUN: execute 0x42::keys::put --args 0x42 --commit
// CHECK: writes: create 0x42::keys::Tag<|&u8|(u8) has copy + drop> at 0x42 0x0100000000000000; create 0x42::keys::Tag<|&0x42::keys::Inner, &vector<u64>|(u64, bool) has copy + drop + store> at 0x42 0x0300000000000000; create 0x42::keys::Tag<|&mut u8|(u8) has copy + drop> at 0x42 0x0200000000000000

// RUN: execute 0x42::keys::read --args 0x42
// CHECK: results: 123

// RUN: execute 0x42::keys::bump --args 0x42 --commit
// CHECK: writes: modify 0x42::keys::Tag<|&u8|(u8) has copy + drop> at 0x42 0x0807060504030201; modify 0x42::keys::Tag<|&0x42::keys::Inner, &vector<u64>|(u64, bool) has copy + drop + store> at 0x42 0x0900000000000000

// RUN: execute 0x42::keys::read --args 0x42
// CHECK: results: 7262385979038285629

// RUN: execute 0x42::keys::take --args 0x42 --commit
// CHECK: results: 72623859790382865 | writes: delete 0x42::keys::Tag<|&u8|(u8) has copy + drop> at 0x42; delete 0x42::keys::Tag<|&0x42::keys::Inner, &vector<u64>|(u64, bool) has copy + drop + store> at 0x42
