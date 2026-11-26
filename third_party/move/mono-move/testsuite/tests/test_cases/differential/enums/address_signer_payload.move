// Enum variants with `address` and `signer` fields after narrow fields, every field
// distinct and addresses using all 32 bytes. Checked on copies, equality, vectors across
// collections, a committed resource, and BCS bytes.

// RUN: publish
module 0x42::payloads {
    use std::signer;
    use std::vector;

    enum Who has copy, drop, store {
        Nobody,
        One { flag: bool, addr: address },
        Two { tag: u8, first: address, second: address, tail: u16 },
    }

    enum Auth has drop {
        Master { tag: u8, account: signer },
        Delegated { flag: bool, owner: address, account: signer, nonce: u16 },
        Anon { addr: address },
    }

    struct Book has key { items: vector<Who>, current: Who }

    // (variant, flag or tag, first address, second address, tail)
    fun describe(value: &Who): (u8, u8, address, address, u16) {
        match (value) {
            Who::Nobody => (0, 0, @0x0, @0x0, 0),
            Who::One { flag, addr } => (1, if (*flag) 1 else 0, *addr, @0x0, 0),
            Who::Two { tag, first, second, tail } => (2, *tag, *first, *second, *tail),
        }
    }

    public fun copy_write(alpha: address, beta: address): (u8, address, address, u16, u8, address, address, u16) {
        let orig = Who::Two { tag: 0xa5, first: alpha, second: beta, tail: 0xbeef };
        let dup = orig;
        match (&mut dup) {
            Who::Two { tag, first, second, tail } => {
                *tag = 0x5a;
                *first = beta;
                *second = alpha;
                *tail = 0x1234;
            },
            _ => abort 1,
        };
        let (_, orig_tag, orig_first, orig_second, orig_tail) = describe(&orig);
        let (_, dup_tag, dup_first, dup_second, dup_tail) = describe(&dup);
        (orig_tag, orig_first, orig_second, orig_tail, dup_tag, dup_first, dup_second, dup_tail)
    }

    // `alpha_low` differs from `alpha` in its last byte, `beta_high` from `beta` in its first.
    public fun address_equality(alpha: address, beta: address, alpha_low: address, beta_high: address): (bool, bool, bool) {
        let base = Who::Two { tag: 0xa5, first: alpha, second: beta, tail: 0xbeef };
        let same = Who::Two { tag: 0xa5, first: alpha, second: beta, tail: 0xbeef };
        let first_differs = Who::Two { tag: 0xa5, first: alpha_low, second: beta, tail: 0xbeef };
        let second_differs = Who::Two { tag: 0xa5, first: alpha, second: beta_high, tail: 0xbeef };
        (base == same, base == first_differs, base == second_differs)
    }

    public fun vector_gc(alpha: address, beta: address, gamma: address): (address, u8, address, address, u16, bool) {
        let list = vector[Who::One { flag: true, addr: alpha }, Who::Two { tag: 1, first: beta, second: gamma, tail: 2 }, Who::Nobody];
        0x0::test_utils::force_gc();
        match (vector::borrow_mut(&mut list, 1)) {
            Who::Two { tag, first, second: _, tail } => {
                *first = alpha;
                *tag = 0x77;
                *tail = 0xfffe;
            },
            _ => abort 1,
        };
        0x0::test_utils::force_gc();
        let snapshot = *vector::borrow(&list, 1);
        0x0::test_utils::force_gc();
        let (_, _, one_addr, _, _) = describe(vector::borrow(&list, 0));
        let (_, tag, first, second, tail) = describe(vector::borrow(&list, 1));
        (one_addr, tag, first, second, tail, snapshot == *vector::borrow(&list, 1))
    }

    // (variant, signer address, owner, tag or nonce, flag)
    fun auth_info(auth: &Auth): (u8, address, address, u16, bool) {
        match (auth) {
            Auth::Master { tag, account } => (0, signer::address_of(account), @0x0, (*tag as u16), false),
            Auth::Delegated { flag, owner, account, nonce } => (1, signer::address_of(account), *owner, *nonce, *flag),
            Auth::Anon { addr } => (2, *addr, @0x0, 0, false),
        }
    }

    public fun signers_gc(first: signer, second: signer, alpha: address): (address, u16, address, address, address, u16, bool, address) {
        let list = vector[
            Auth::Master { tag: 0xc3, account: first },
            Auth::Anon { addr: alpha },
            Auth::Delegated { flag: true, owner: alpha, account: second, nonce: 0x3344 },
        ];
        0x0::test_utils::force_gc();
        let extra = vector::pop_back(&mut list);
        0x0::test_utils::force_gc();
        let (_, master, _, master_tag, _) = auth_info(vector::borrow(&list, 0));
        let (_, anon, _, _, _) = auth_info(vector::borrow(&list, 1));
        let (_, delegated, owner, nonce, flag) = auth_info(&extra);
        (master, master_tag, anon, delegated, owner, nonce, flag, signer::address_of(&signer_of(extra)))
    }

    fun signer_of(auth: Auth): signer {
        match (auth) {
            Auth::Delegated { flag: _, owner: _, account, nonce: _ } => account,
            _ => abort 2,
        }
    }

    public fun signer_bytes(account: signer, alpha: address): vector<u8> {
        std::bcs::to_bytes(&Auth::Delegated { flag: true, owner: alpha, account, nonce: 0x3344 })
    }

    public fun store(account: signer, alpha: address, beta: address) {
        move_to(&account, Book {
            items: vector[Who::One { flag: true, addr: alpha }, Who::Nobody, Who::Two { tag: 0x11, first: beta, second: alpha, tail: 0x2233 }],
            current: Who::One { flag: false, addr: beta },
        });
    }

    // Every field of `current` and of the last item.
    public fun read(owner: address): (u8, u8, address, address, u16, u8, u8, address, address, u16) {
        let book = borrow_global<Book>(owner);
        let (cur_kind, cur_narrow, cur_first, cur_second, cur_tail) = describe(&book.current);
        let (last_kind, last_tag, last_first, last_second, last_tail) = describe(vector::borrow(&book.items, 2));
        (cur_kind, cur_narrow, cur_first, cur_second, cur_tail, last_kind, last_tag, last_first, last_second, last_tail)
    }

    // `current` changes variant; the last item is written in place.
    public fun rewrite(owner: address, gamma: address, delta: address) {
        let book = borrow_global_mut<Book>(owner);
        book.current = Who::Two { tag: 0x44, first: gamma, second: delta, tail: 0x5566 };
        match (vector::borrow_mut(&mut book.items, 2)) {
            Who::Two { tag: _, first: _, second, tail: _ } => *second = gamma,
            _ => abort 3,
        };
    }
}

// RUN: execute 0x42::payloads::copy_write --args 0xa1b2c3d4e5f60718293a4b5c6d7e8f90a1b2c3d4e5f60718293a4b5c6d7e8f90, 0x1f2e3d4c5b6a79880123456789abcdef0fedcba9876543210f1e2d3c4b5a6978
// CHECK: results: 165, 0xa1b2c3d4e5f60718293a4b5c6d7e8f90a1b2c3d4e5f60718293a4b5c6d7e8f90, 0x1f2e3d4c5b6a79880123456789abcdef0fedcba9876543210f1e2d3c4b5a6978, 48879, 90, 0x1f2e3d4c5b6a79880123456789abcdef0fedcba9876543210f1e2d3c4b5a6978, 0xa1b2c3d4e5f60718293a4b5c6d7e8f90a1b2c3d4e5f60718293a4b5c6d7e8f90, 4660

// RUN: execute 0x42::payloads::address_equality --args 0xa1b2c3d4e5f60718293a4b5c6d7e8f90a1b2c3d4e5f60718293a4b5c6d7e8f90, 0x1f2e3d4c5b6a79880123456789abcdef0fedcba9876543210f1e2d3c4b5a6978, 0xa1b2c3d4e5f60718293a4b5c6d7e8f90a1b2c3d4e5f60718293a4b5c6d7e8f91, 0x9f2e3d4c5b6a79880123456789abcdef0fedcba9876543210f1e2d3c4b5a6978
// CHECK: results: true, false, false

// RUN: execute 0x42::payloads::vector_gc --args 0xa1b2c3d4e5f60718293a4b5c6d7e8f90a1b2c3d4e5f60718293a4b5c6d7e8f90, 0x1f2e3d4c5b6a79880123456789abcdef0fedcba9876543210f1e2d3c4b5a6978, 0xc0ffee00000000000000000000000000000000000000000000000000000000ee
// CHECK: results: 0xa1b2c3d4e5f60718293a4b5c6d7e8f90a1b2c3d4e5f60718293a4b5c6d7e8f90, 119, 0xa1b2c3d4e5f60718293a4b5c6d7e8f90a1b2c3d4e5f60718293a4b5c6d7e8f90, 0xc0ffee00000000000000000000000000000000000000000000000000000000ee, 65534, true
// CHECK-GC-COUNT: 3

// RUN: execute 0x42::payloads::signers_gc --args 0xd00d0000000000000000000000000000000000000000000000000000000000d1, 0xe11e0000000000000000000000000000000000000000000000000000000000e2, 0xa1b2c3d4e5f60718293a4b5c6d7e8f90a1b2c3d4e5f60718293a4b5c6d7e8f90
// CHECK: results: 0xd00d0000000000000000000000000000000000000000000000000000000000d1, 195, 0xa1b2c3d4e5f60718293a4b5c6d7e8f90a1b2c3d4e5f60718293a4b5c6d7e8f90, 0xe11e0000000000000000000000000000000000000000000000000000000000e2, 0xa1b2c3d4e5f60718293a4b5c6d7e8f90a1b2c3d4e5f60718293a4b5c6d7e8f90, 13124, true, 0xe11e0000000000000000000000000000000000000000000000000000000000e2
// CHECK-GC-COUNT: 2

// RUN: execute 0x42::payloads::signer_bytes --args 0xe11e0000000000000000000000000000000000000000000000000000000000e2, 0xa1b2c3d4e5f60718293a4b5c6d7e8f90a1b2c3d4e5f60718293a4b5c6d7e8f90
// CHECK: results: 0x0101a1b2c3d4e5f60718293a4b5c6d7e8f90a1b2c3d4e5f60718293a4b5c6d7e8f90e11e0000000000000000000000000000000000000000000000000000000000e24433

// RUN: execute 0x42::payloads::store --args 0x42, 0xa1b2c3d4e5f60718293a4b5c6d7e8f90a1b2c3d4e5f60718293a4b5c6d7e8f90, 0x1f2e3d4c5b6a79880123456789abcdef0fedcba9876543210f1e2d3c4b5a6978 --commit
// CHECK: writes: create 0x42::payloads::Book at 0x42 0x030101a1b2c3d4e5f60718293a4b5c6d7e8f90a1b2c3d4e5f60718293a4b5c6d7e8f900002111f2e3d4c5b6a79880123456789abcdef0fedcba9876543210f1e2d3c4b5a6978a1b2c3d4e5f60718293a4b5c6d7e8f90a1b2c3d4e5f60718293a4b5c6d7e8f90332201001f2e3d4c5b6a79880123456789abcdef0fedcba9876543210f1e2d3c4b5a6978

// RUN: execute 0x42::payloads::read --args 0x42
// CHECK: results: 1, 0, 0x1f2e3d4c5b6a79880123456789abcdef0fedcba9876543210f1e2d3c4b5a6978, 0x0, 0, 2, 17, 0x1f2e3d4c5b6a79880123456789abcdef0fedcba9876543210f1e2d3c4b5a6978, 0xa1b2c3d4e5f60718293a4b5c6d7e8f90a1b2c3d4e5f60718293a4b5c6d7e8f90, 8755

// RUN: execute 0x42::payloads::rewrite --args 0x42, 0xc0ffee00000000000000000000000000000000000000000000000000000000ee, 0x7d6c5b4a39281706f5e4d3c2b1a09f8e7d6c5b4a39281706f5e4d3c2b1a09f8e --commit
// CHECK: writes: modify 0x42::payloads::Book at 0x42 0x030101a1b2c3d4e5f60718293a4b5c6d7e8f90a1b2c3d4e5f60718293a4b5c6d7e8f900002111f2e3d4c5b6a79880123456789abcdef0fedcba9876543210f1e2d3c4b5a6978c0ffee00000000000000000000000000000000000000000000000000000000ee33220244c0ffee00000000000000000000000000000000000000000000000000000000ee7d6c5b4a39281706f5e4d3c2b1a09f8e7d6c5b4a39281706f5e4d3c2b1a09f8e6655

// RUN: execute 0x42::payloads::read --args 0x42
// CHECK: results: 2, 68, 0xc0ffee00000000000000000000000000000000000000000000000000000000ee, 0x7d6c5b4a39281706f5e4d3c2b1a09f8e7d6c5b4a39281706f5e4d3c2b1a09f8e, 21862, 2, 17, 0x1f2e3d4c5b6a79880123456789abcdef0fedcba9876543210f1e2d3c4b5a6978, 0xc0ffee00000000000000000000000000000000000000000000000000000000ee, 8755
