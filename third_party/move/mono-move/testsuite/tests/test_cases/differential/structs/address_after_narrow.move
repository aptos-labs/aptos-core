// Address fields right after narrow fields (`bool`, `u8`) and a `u256` after a `u8`, with
// every field distinct and addresses using all 32 bytes. Checked through accessors whose
// lowering the golden pins, on copies, in vectors across collections, and in a committed
// resource.

// RUN: publish --print(micro-ops)
module 0x42::addr_offsets {
    struct Slot has copy, drop { flag: bool, addr: address, tag: u8, who: address, tail: u16 }

    struct Tagged has copy, drop { tag: u8, big: u256 }

    fun flag_of(slot: &Slot): bool { slot.flag }
    fun addr_of(slot: &Slot): address { slot.addr }
    fun tag_of(slot: &Slot): u8 { slot.tag }
    fun who_of(slot: &Slot): address { slot.who }
    fun tail_of(slot: &Slot): u16 { slot.tail }
    fun set_addr(slot: &mut Slot, addr: address) { slot.addr = addr }
    fun set_who(slot: &mut Slot, who: address) { slot.who = who }
    fun big_of(value: &Tagged): u256 { value.big }

    public fun accessors(alpha: address, beta: address, gamma: address, big: u256): (bool, address, u8, address, u16, u8, u256) {
        let slot = Slot { flag: true, addr: alpha, tag: 0xa5, who: gamma, tail: 0xbeef };
        set_addr(&mut slot, beta);
        set_who(&mut slot, alpha);
        let value = Tagged { tag: 0x5a, big };
        (flag_of(&slot), addr_of(&slot), tag_of(&slot), who_of(&slot), tail_of(&slot), value.tag, big_of(&value))
    }
}

// RUN: publish
module 0x42::addr_layout {
    use std::vector;

    struct Slot has copy, drop, store { flag: bool, addr: address, tag: u8, who: address, tail: u16 }

    struct Holder has key { slots: vector<Slot>, first: Slot }

    fun slot(flag: bool, addr: address, tag: u8, who: address, tail: u16): Slot {
        Slot { flag, addr, tag, who, tail }
    }

    fun fields(value: &Slot): (bool, address, u8, address, u16) {
        (value.flag, value.addr, value.tag, value.who, value.tail)
    }

    // The copy is rewritten through field references; the original keeps its values.
    public fun copy_write(alpha: address, beta: address): (bool, address, u8, address, u16, bool, address, u8, address, u16) {
        let orig = slot(true, alpha, 0xa5, beta, 0xbeef);
        let dup = orig;
        let flag = &mut dup.flag;
        *flag = false;
        let addr = &mut dup.addr;
        *addr = beta;
        dup.tag = 0x5a;
        let who = &mut dup.who;
        *who = alpha;
        dup.tail = 0x1234;
        let (orig_flag, orig_addr, orig_tag, orig_who, orig_tail) = fields(&orig);
        let (dup_flag, dup_addr, dup_tag, dup_who, dup_tail) = fields(&dup);
        (orig_flag, orig_addr, orig_tag, orig_who, orig_tail, dup_flag, dup_addr, dup_tag, dup_who, dup_tail)
    }

    public fun vector_gc(alpha: address, beta: address, gamma: address): (address, bool, address, u8, address, u16, address, bool) {
        let list = vector[slot(true, alpha, 1, beta, 10), slot(false, beta, 2, gamma, 20), slot(true, gamma, 3, alpha, 30)];
        0x0::test_utils::force_gc();
        let elem = vector::borrow_mut(&mut list, 1);
        elem.addr = alpha;
        elem.flag = true;
        elem.tag = 0x77;
        elem.tail = 0xfffe;
        0x0::test_utils::force_gc();
        let snapshot = *vector::borrow(&list, 1);
        0x0::test_utils::force_gc();
        let (mid_flag, mid_addr, mid_tag, mid_who, mid_tail) = fields(vector::borrow(&list, 1));
        (
            vector::borrow(&list, 0).who,
            mid_flag,
            mid_addr,
            mid_tag,
            mid_who,
            mid_tail,
            vector::borrow(&list, 2).addr,
            snapshot == *vector::borrow(&list, 1),
        )
    }

    public fun store(account: signer, alpha: address, beta: address) {
        move_to(&account, Holder {
            slots: vector[slot(true, alpha, 0x11, beta, 0x2233), slot(false, beta, 0x44, alpha, 0x5566)],
            first: slot(true, beta, 0x77, alpha, 0x8899),
        });
    }

    // Every field of `first` and of the second element.
    public fun read(owner: address): (bool, address, u8, address, u16, bool, address, u8, address, u16) {
        let holder = borrow_global<Holder>(owner);
        let (first_flag, first_addr, first_tag, first_who, first_tail) = fields(&holder.first);
        let (second_flag, second_addr, second_tag, second_who, second_tail) = fields(vector::borrow(&holder.slots, 1));
        (first_flag, first_addr, first_tag, first_who, first_tail, second_flag, second_addr, second_tag, second_who, second_tail)
    }

    public fun rewrite(owner: address, gamma: address, delta: address) {
        let holder = borrow_global_mut<Holder>(owner);
        let second = vector::borrow_mut(&mut holder.slots, 1);
        second.addr = gamma;
        second.flag = true;
        holder.first.who = delta;
        holder.first.tag = 0;
    }
}

// RUN: execute 0x42::addr_offsets::accessors --args 0xa1b2c3d4e5f60718293a4b5c6d7e8f90a1b2c3d4e5f60718293a4b5c6d7e8f90, 0x1f2e3d4c5b6a79880123456789abcdef0fedcba9876543210f1e2d3c4b5a6978, 0x3c4d5e6f708192a3b4c5d6e7f8091a2b2b1a09f8e7d6c5b4a39281706f5e4d3c, 514631507721405312519378913364952599457899916736173488040697764812573303280
// CHECK: results: true, 0x1f2e3d4c5b6a79880123456789abcdef0fedcba9876543210f1e2d3c4b5a6978, 165, 0xa1b2c3d4e5f60718293a4b5c6d7e8f90a1b2c3d4e5f60718293a4b5c6d7e8f90, 48879, 90, 514631507721405312519378913364952599457899916736173488040697764812573303280

// RUN: execute 0x42::addr_layout::copy_write --args 0xa1b2c3d4e5f60718293a4b5c6d7e8f90a1b2c3d4e5f60718293a4b5c6d7e8f90, 0x1f2e3d4c5b6a79880123456789abcdef0fedcba9876543210f1e2d3c4b5a6978
// CHECK: results: true, 0xa1b2c3d4e5f60718293a4b5c6d7e8f90a1b2c3d4e5f60718293a4b5c6d7e8f90, 165, 0x1f2e3d4c5b6a79880123456789abcdef0fedcba9876543210f1e2d3c4b5a6978, 48879, false, 0x1f2e3d4c5b6a79880123456789abcdef0fedcba9876543210f1e2d3c4b5a6978, 90, 0xa1b2c3d4e5f60718293a4b5c6d7e8f90a1b2c3d4e5f60718293a4b5c6d7e8f90, 4660

// RUN: execute 0x42::addr_layout::vector_gc --args 0xa1b2c3d4e5f60718293a4b5c6d7e8f90a1b2c3d4e5f60718293a4b5c6d7e8f90, 0x1f2e3d4c5b6a79880123456789abcdef0fedcba9876543210f1e2d3c4b5a6978, 0xc0ffee00000000000000000000000000000000000000000000000000000000ee
// CHECK: results: 0x1f2e3d4c5b6a79880123456789abcdef0fedcba9876543210f1e2d3c4b5a6978, true, 0xa1b2c3d4e5f60718293a4b5c6d7e8f90a1b2c3d4e5f60718293a4b5c6d7e8f90, 119, 0xc0ffee00000000000000000000000000000000000000000000000000000000ee, 65534, 0xc0ffee00000000000000000000000000000000000000000000000000000000ee, true
// CHECK-GC-COUNT: 3

// RUN: execute 0x42::addr_layout::store --args 0x42, 0xa1b2c3d4e5f60718293a4b5c6d7e8f90a1b2c3d4e5f60718293a4b5c6d7e8f90, 0x1f2e3d4c5b6a79880123456789abcdef0fedcba9876543210f1e2d3c4b5a6978 --commit
// CHECK: writes: create 0x42::addr_layout::Holder at 0x42 0x0201a1b2c3d4e5f60718293a4b5c6d7e8f90a1b2c3d4e5f60718293a4b5c6d7e8f90111f2e3d4c5b6a79880123456789abcdef0fedcba9876543210f1e2d3c4b5a69783322001f2e3d4c5b6a79880123456789abcdef0fedcba9876543210f1e2d3c4b5a697844a1b2c3d4e5f60718293a4b5c6d7e8f90a1b2c3d4e5f60718293a4b5c6d7e8f906655011f2e3d4c5b6a79880123456789abcdef0fedcba9876543210f1e2d3c4b5a697877a1b2c3d4e5f60718293a4b5c6d7e8f90a1b2c3d4e5f60718293a4b5c6d7e8f909988

// RUN: execute 0x42::addr_layout::read --args 0x42
// CHECK: results: true, 0x1f2e3d4c5b6a79880123456789abcdef0fedcba9876543210f1e2d3c4b5a6978, 119, 0xa1b2c3d4e5f60718293a4b5c6d7e8f90a1b2c3d4e5f60718293a4b5c6d7e8f90, 34969, false, 0x1f2e3d4c5b6a79880123456789abcdef0fedcba9876543210f1e2d3c4b5a6978, 68, 0xa1b2c3d4e5f60718293a4b5c6d7e8f90a1b2c3d4e5f60718293a4b5c6d7e8f90, 21862

// RUN: execute 0x42::addr_layout::rewrite --args 0x42, 0xc0ffee00000000000000000000000000000000000000000000000000000000ee, 0x7d6c5b4a39281706f5e4d3c2b1a09f8e7d6c5b4a39281706f5e4d3c2b1a09f8e --commit
// CHECK: writes: modify 0x42::addr_layout::Holder at 0x42 0x0201a1b2c3d4e5f60718293a4b5c6d7e8f90a1b2c3d4e5f60718293a4b5c6d7e8f90111f2e3d4c5b6a79880123456789abcdef0fedcba9876543210f1e2d3c4b5a6978332201c0ffee00000000000000000000000000000000000000000000000000000000ee44a1b2c3d4e5f60718293a4b5c6d7e8f90a1b2c3d4e5f60718293a4b5c6d7e8f906655011f2e3d4c5b6a79880123456789abcdef0fedcba9876543210f1e2d3c4b5a6978007d6c5b4a39281706f5e4d3c2b1a09f8e7d6c5b4a39281706f5e4d3c2b1a09f8e9988

// RUN: execute 0x42::addr_layout::read --args 0x42
// CHECK: results: true, 0x1f2e3d4c5b6a79880123456789abcdef0fedcba9876543210f1e2d3c4b5a6978, 0, 0x7d6c5b4a39281706f5e4d3c2b1a09f8e7d6c5b4a39281706f5e4d3c2b1a09f8e, 34969, true, 0xc0ffee00000000000000000000000000000000000000000000000000000000ee, 68, 0xa1b2c3d4e5f60718293a4b5c6d7e8f90a1b2c3d4e5f60718293a4b5c6d7e8f90, 21862
