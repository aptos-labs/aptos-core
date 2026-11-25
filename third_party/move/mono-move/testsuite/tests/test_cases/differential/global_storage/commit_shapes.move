// Write-set bytes and resource keys for each field type shape. Every key is
// created, reloaded from storage in a later step, modified and deleted, with
// the writes pinned on the mutating step.

// RUN: publish
module 0x42::shapes {
    use std::string::{Self, String};
    use std::option::{Self, Option};
    use std::vector;

    struct Narrow has key, drop { a: u8, b: u64, c: bool, d: u16, e: u128, f: u256, g: address, h: u32 }
    struct Signed has key, drop { a: i8, b: i128, c: i16, d: i256, e: i64, f: i32 }
    struct Inner has store, drop, copy { x: u8, y: u64 }
    struct Nested has key, drop {
        tag: u32,
        inner: Inner,
        v: vector<Inner>,
        vv: vector<vector<u8>>,
        s: String,
        o: Option<u64>,
        n: Option<u64>,
    }
    enum E has key, store, drop { Unit, Small { x: u8 }, Big { y: u256, z: vector<u64> } }
    struct G<T> has key, drop { t: T }
    struct Tag<phantom T> has key, drop { n: u64 }

    public fun put_narrow(account: signer) {
        move_to(&account, Narrow { a: 1, b: 2, c: true, d: 3, e: 4, f: 5, g: @0x7, h: 0xdeadbeef })
    }

    public fun put_signed(account: signer) {
        move_to(&account, Signed { a: -1, b: -2, c: -3, d: -4, e: -5, f: -6 })
    }

    public fun put_nested(account: signer) {
        move_to(&account, Nested {
            tag: 0x01020304,
            inner: Inner { x: 1, y: 2 },
            v: vector[Inner { x: 3, y: 4 }, Inner { x: 5, y: 6 }],
            vv: vector[b"ab", b""],
            s: string::utf8(b"hi"),
            o: option::some(9),
            n: option::none(),
        })
    }

    public fun put_enum_unit(account: signer) { move_to(&account, E::Unit) }
    public fun put_enum_small(account: signer) { move_to(&account, E::Small { x: 0xff }) }
    public fun put_enum_big(account: signer) { move_to(&account, E::Big { y: 7, z: vector[1, 2] }) }

    public fun put_generic(account: signer) {
        move_to(&account, G<u128> { t: 1 });
        move_to(&account, G<vector<Inner>> { t: vector[Inner { x: 8, y: 9 }] });
        move_to(&account, G<E> { t: E::Small { x: 3 } });
        move_to(&account, Tag<|&mut u8|u8 has copy + drop> { n: 1 });
        move_to(&account, Tag<G<address>> { n: 2 });
    }

    public fun read_narrow(addr: address): (u8, u64, bool, u16, u128, u256, address, u32) {
        let narrow = borrow_global<Narrow>(addr);
        (narrow.a, narrow.b, narrow.c, narrow.d, narrow.e, narrow.f, narrow.g, narrow.h)
    }

    public fun read_signed(addr: address): (i8, i128, i16, i256, i64, i32) {
        let signed = borrow_global<Signed>(addr);
        (signed.a, signed.b, signed.c, signed.d, signed.e, signed.f)
    }

    fun opt_or_zero(opt: &Option<u64>): u64 {
        if (option::is_some(opt)) *option::borrow(opt) else 0
    }

    public fun read_nested(addr: address): (u32, u8, u64, u64, u64, vector<u8>, vector<u8>, String, u64, u64) {
        let nested = borrow_global<Nested>(addr);
        let last = vector::length(&nested.v) - 1;
        (
            nested.tag,
            nested.inner.x,
            nested.inner.y,
            vector::length(&nested.v),
            vector::borrow(&nested.v, last).y,
            *vector::borrow(&nested.vv, 0),
            *vector::borrow(&nested.vv, 1),
            nested.s,
            opt_or_zero(&nested.o),
            opt_or_zero(&nested.n),
        )
    }

    fun describe(value: &E): vector<u64> {
        match (value) {
            E::Unit => vector[0],
            E::Small { x: byte } => vector[1, (*byte as u64)],
            E::Big { y: wide, z: values } => {
                let out = vector[2, ((*wide % 1000) as u64), ((*wide >> 248) as u64)];
                vector::append(&mut out, *values);
                out
            },
        }
    }

    public fun read_enums(): vector<u64> {
        let out = describe(borrow_global<E>(@0x43));
        vector::append(&mut out, describe(borrow_global<E>(@0x44)));
        vector::append(&mut out, describe(borrow_global<E>(@0x45)));
        out
    }

    public fun read_generic(addr: address): (u128, u64, vector<u64>, u64, u64) {
        (
            borrow_global<G<u128>>(addr).t,
            vector::borrow(&borrow_global<G<vector<Inner>>>(addr).t, 0).y,
            describe(&borrow_global<G<E>>(addr).t),
            borrow_global<Tag<|&mut u8|u8 has copy + drop>>(addr).n,
            borrow_global<Tag<G<address>>>(addr).n,
        )
    }

    public fun bump_narrow(addr: address) {
        let narrow = borrow_global_mut<Narrow>(addr);
        narrow.a = 0xfe;
        narrow.c = false;
        narrow.e = narrow.e << 100;
        narrow.f = narrow.f << 200;
        narrow.g = @0xcafe;
    }

    public fun negate_signed(addr: address) {
        let signed = borrow_global_mut<Signed>(addr);
        signed.a = -signed.a;
        signed.b = signed.b * 1000000000000000000000;
        signed.d = -signed.d;
        signed.f = -2147483648;
    }

    public fun grow_nested(addr: address) {
        let nested = borrow_global_mut<Nested>(addr);
        vector::push_back(&mut nested.v, Inner { x: 7, y: 8 });
        vector::push_back(vector::borrow_mut(&mut nested.vv, 1), 0x63);
        string::append(&mut nested.s, string::utf8(b"!"));
        option::extract(&mut nested.o);
        option::fill(&mut nested.n, 11);
    }

    // Unit -> Big, Small -> Unit, Big -> Small; also a variant field write in place.
    public fun rotate_enums() {
        *borrow_global_mut<E>(@0x43) = E::Big { y: (1 << 255) + 5, z: vector[] };
        *borrow_global_mut<E>(@0x44) = E::Unit;
        *borrow_global_mut<E>(@0x45) = E::Small { x: 4 };
        let holder = borrow_global_mut<G<E>>(@0x42);
        match (&mut holder.t) {
            E::Small { x: byte } => *byte = *byte + 1,
            _ => abort 1,
        };
    }

    public fun bump_generic(addr: address) {
        let holder = borrow_global_mut<G<u128>>(addr);
        holder.t = holder.t + (1 << 127);
        vector::push_back(&mut borrow_global_mut<G<vector<Inner>>>(addr).t, Inner { x: 10, y: 11 });
        borrow_global_mut<Tag<|&mut u8|u8 has copy + drop>>(addr).n = 0xfffffffffffffffe;
        borrow_global_mut<Tag<G<address>>>(addr).n = 3;
    }

    public fun remove_enums(): u64 {
        let total = 0;
        let addrs = vector[@0x43, @0x44, @0x45];
        vector::for_each(addrs, |addr| total = total * 10 + vector::length(&describe(&move_from<E>(addr))));
        total
    }

    public fun remove_all(addr: address) {
        let Narrow { a: _, b: _, c: _, d: _, e: _, f: _, g: _, h: _ } = move_from<Narrow>(addr);
        move_from<Signed>(addr);
        move_from<Nested>(addr);
        move_from<G<u128>>(addr);
        move_from<G<vector<Inner>>>(addr);
        move_from<G<E>>(addr);
        move_from<Tag<|&mut u8|u8 has copy + drop>>(addr);
        move_from<Tag<G<address>>>(addr);
    }
}

// RUN: execute 0x42::shapes::put_narrow --args 0x42 --commit
// CHECK: writes: create 0x42::shapes::Narrow at 0x42 0x0102000000000000000103000400000000000000000000000000000005000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000007efbeadde

// RUN: execute 0x42::shapes::put_signed --args 0x42 --commit
// CHECK: writes: create 0x42::shapes::Signed at 0x42 0xfffefffffffffffffffffffffffffffffffdfffcfffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffbfffffffffffffffaffffff

// RUN: execute 0x42::shapes::put_nested --args 0x42 --commit
// CHECK: writes: create 0x42::shapes::Nested at 0x42 0x0403020101020000000000000002030400000000000000050600000000000000020261620002686901090000000000000000

// RUN: execute 0x42::shapes::put_enum_unit --args 0x43 --commit
// CHECK: writes: create 0x42::shapes::E at 0x43 0x00

// RUN: execute 0x42::shapes::put_enum_small --args 0x44 --commit
// CHECK: writes: create 0x42::shapes::E at 0x44 0x01ff

// RUN: execute 0x42::shapes::put_enum_big --args 0x45 --commit
// CHECK: writes: create 0x42::shapes::E at 0x45 0x0207000000000000000000000000000000000000000000000000000000000000000201000000000000000200000000000000

// RUN: execute 0x42::shapes::put_generic --args 0x42 --commit
// CHECK: writes: create 0x42::shapes::G<u128> at 0x42 0x01000000000000000000000000000000; create 0x42::shapes::G<vector<0x42::shapes::Inner>> at 0x42 0x01080900000000000000; create 0x42::shapes::G<0x42::shapes::E> at 0x42 0x0103; create 0x42::shapes::Tag<0x42::shapes::G<address>> at 0x42 0x0200000000000000; create 0x42::shapes::Tag<|&mut u8|(u8) has copy + drop> at 0x42 0x0100000000000000

// Each read decodes bytes the earlier steps committed.
// RUN: execute 0x42::shapes::read_narrow --args 0x42
// CHECK: results: 1, 2, true, 3, 4, 5, 0x7, 3735928559

// RUN: execute 0x42::shapes::read_signed --args 0x42
// CHECK: results: -1, -2, -3, -4, -5, -6

// RUN: execute 0x42::shapes::read_nested --args 0x42
// CHECK: results: 16909060, 1, 2, 2, 6, 0x6162, 0x, "hi", 9, 0

// RUN: execute 0x42::shapes::read_enums
// CHECK: results: [0, 1, 255, 2, 7, 0, 1, 2]

// RUN: execute 0x42::shapes::read_generic --args 0x42
// CHECK: results: 1, 9, [1, 3], 1, 2

// RUN: execute 0x42::shapes::bump_narrow --args 0x42 --commit
// CHECK: writes: modify 0x42::shapes::Narrow at 0x42 0xfe0200000000000000000300000000000000000000000000400000000000000000000000000000000000000000000000000000000005000000000000000000000000000000000000000000000000000000000000000000000000cafeefbeadde

// RUN: execute 0x42::shapes::negate_signed --args 0x42 --commit
// CHECK: writes: modify 0x42::shapes::Signed at 0x42 0x010000c04274a46c9493fffffffffffffffdff0400000000000000000000000000000000000000000000000000000000000000fbffffffffffffff00000080

// RUN: execute 0x42::shapes::grow_nested --args 0x42 --commit
// CHECK: writes: modify 0x42::shapes::Nested at 0x42 0x04030201010200000000000000030304000000000000000506000000000000000708000000000000000202616201630368692100010b00000000000000

// RUN: execute 0x42::shapes::rotate_enums --commit
// CHECK: writes: modify 0x42::shapes::G<0x42::shapes::E> at 0x42 0x0104; modify 0x42::shapes::E at 0x43 0x02050000000000000000000000000000000000000000000000000000000000008000; modify 0x42::shapes::E at 0x44 0x00; modify 0x42::shapes::E at 0x45 0x0104

// RUN: execute 0x42::shapes::read_nested --args 0x42
// CHECK: results: 16909060, 1, 2, 3, 8, 0x6162, 0x63, "hi!", 0, 11

// RUN: execute 0x42::shapes::read_enums
// CHECK: results: [2, 973, 128, 0, 1, 4]

// RUN: execute 0x42::shapes::bump_generic --args 0x42 --commit
// CHECK: writes: modify 0x42::shapes::G<u128> at 0x42 0x01000000000000000000000000000080; modify 0x42::shapes::G<vector<0x42::shapes::Inner>> at 0x42 0x020809000000000000000a0b00000000000000; modify 0x42::shapes::Tag<0x42::shapes::G<address>> at 0x42 0x0300000000000000; modify 0x42::shapes::Tag<|&mut u8|(u8) has copy + drop> at 0x42 0xfeffffffffffffff

// RUN: execute 0x42::shapes::read_generic --args 0x42
// CHECK: results: 170141183460469231731687303715884105729, 9, [1, 4], 18446744073709551614, 3

// RUN: execute 0x42::shapes::remove_enums --commit
// CHECK: results: 312 | writes: delete 0x42::shapes::E at 0x43; delete 0x42::shapes::E at 0x44; delete 0x42::shapes::E at 0x45

// RUN: execute 0x42::shapes::remove_all --args 0x42 --commit
// CHECK: writes: delete 0x42::shapes::G<u128> at 0x42; delete 0x42::shapes::G<vector<0x42::shapes::Inner>> at 0x42; delete 0x42::shapes::G<0x42::shapes::E> at 0x42; delete 0x42::shapes::Narrow at 0x42; delete 0x42::shapes::Nested at 0x42; delete 0x42::shapes::Signed at 0x42; delete 0x42::shapes::Tag<0x42::shapes::G<address>> at 0x42; delete 0x42::shapes::Tag<|&mut u8|(u8) has copy + drop> at 0x42
