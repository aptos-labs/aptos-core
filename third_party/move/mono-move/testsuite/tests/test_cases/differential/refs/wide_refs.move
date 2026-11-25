// References to u128, u256 and address values in locals, struct fields and vector
// elements: read, written, compared and passed to callees. The constants are nonzero in
// every 64-bit word, the bumps add to every word, and one compared address pair differs
// only in its first byte, so a truncated or partial copy changes a result.

// RUN: publish
module 0x42::wide_refs {
    use std::vector;

    struct W has copy, drop { tag: u8, big: u256, mid: u128, who: address }

    const BIG: u256 = 0x0123456789abcdeffedcba98765432100f1e2d3c4b5a69788796a5b4c3d2e1f0;
    const MID: u128 = 0x0123456789abcdeffedcba9876543210;

    fun bump128(target: &mut u128) { *target = *target + (1u128 << 100) + 1 }
    fun bump256(target: &mut u256) { *target = *target + (1u256 << 192) + (1u256 << 128) + (1u256 << 64) + 1 }
    fun set_addr(target: &mut address, addr: address) { *target = addr }
    fun read256(source: &u256): u256 { *source }

    public fun locals(): (u128, u256, address, bool) {
        let mid: u128 = 7;
        let big: u256 = 1 << 200;
        let addr = @0x1;
        bump128(&mut mid);
        bump256(&mut big);
        set_addr(&mut addr, @0xbeef);
        let other: u256 = (1 << 200) + (1 << 192) + (1 << 128) + (1 << 64) + 1;
        (mid, big, addr, &big == &other)
    }

    public fun fields(): (u128, u256, address, u256, address, bool) {
        let record = W { tag: 1, big: BIG, mid: MID, who: @0x0102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f20 };
        bump128(&mut record.mid);
        bump256(&mut record.big);
        set_addr(&mut record.who, @0xa1b2c3d4e5f60718293a4b5c6d7e8f90a1b2c3d4e5f60718293a4b5c6d7e8f99);
        let record_ref = &record;
        let copied = *record_ref;
        record.big = 0;
        record.who = @0xff02030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f20;
        (copied.mid, copied.big, copied.who, read256(&record.big), record.who, &copied.who == &record.who)
    }

    public fun vector_elems(): (u128, u256, address, bool, bool) {
        let mids = vector[1u128, 2, MID];
        let bigs = vector[BIG, 20];
        let whos = vector[
            @0x0102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f20,
            @0x2,
        ];
        bump128(vector::borrow_mut(&mut mids, 2));
        bump256(vector::borrow_mut(&mut bigs, 0));
        set_addr(vector::borrow_mut(&mut whos, 1), @0xa1b2c3d4e5f60718293a4b5c6d7e8f90a1b2c3d4e5f60718293a4b5c6d7e8f99);
        let same = vector::borrow(&whos, 0) == &@0x0102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f20;
        let high_differs = vector::borrow(&whos, 0) == &@0xff02030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f20;
        (*vector::borrow(&mids, 2), *vector::borrow(&bigs, 0), *vector::borrow(&whos, 1), same, high_differs)
    }
}

// RUN: execute 0x42::wide_refs::locals
// CHECK: results: 1267650600228229401496703205384, 1613215145994376956306138164131291207401787170578762347577345, 0xbeef, true

// RUN: execute 0x42::wide_refs::fields
// CHECK: results: 1512367342854771175561756866386342417, 514631507721405318796480648751633363633971706864778367624874560782085579249, 0xa1b2c3d4e5f60718293a4b5c6d7e8f90a1b2c3d4e5f60718293a4b5c6d7e8f99, 0, 0xff02030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f20, false

// RUN: execute 0x42::wide_refs::vector_elems
// CHECK: results: 1512367342854771175561756866386342417, 514631507721405318796480648751633363633971706864778367624874560782085579249, 0xa1b2c3d4e5f60718293a4b5c6d7e8f90a1b2c3d4e5f60718293a4b5c6d7e8f99, true, false
