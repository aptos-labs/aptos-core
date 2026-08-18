// RUN: publish --print(stackless)

module 0x99::closure_abilities {
    struct Res has store { x: u64 }
    struct Plain has copy, drop, store { x: u64 }
    // Non-phantom: abilities are predicated on T.
    struct Box<T> has copy, drop, store { x: T }
    // Phantom: T never predicates Box2's abilities.
    struct Box2<phantom T> has copy, drop, store { x: u64 }
    enum E has copy, drop { V1 { x: u64 }, V2 { x: bool } }

    fun add3(a: u64, b: u64, c: u64): u64 { a + b + c }
    public fun public_add(a: u64, b: u64): u64 { a + b }
    fun takes_res(r: Res, x: u64): u64 { let Res { x: y } = r; x + y }
    fun takes_plain(p: Plain, x: u64): u64 { let Plain { x: y } = p; x + y }
    fun generic_pick<T: drop>(t: T, other: T, flag: bool): T { if (flag) t else other }

    // Captures nothing: all three parameters remain.
    public fun none_captured(): |u64, u64, u64|u64 has copy + drop { add3 }

    // Captures the leading parameter: two remain.
    public fun one_captured(a: u64): |u64, u64|u64 has copy + drop { |b, c| add3(a, b, c) }

    // Captures every parameter: none remain.
    public fun all_captured(a: u64, b: u64, c: u64): ||u64 has copy + drop { || add3(a, b, c) }

    // Private callee is not storable, so the closure is copy+drop only.
    public fun private_callee(a: u64): |u64|u64 has copy + drop { |b| add3(a, 0, b) }

    // Public callee is storable, so the closure gains store.
    public fun public_callee(a: u64): |u64|u64 has copy + drop + store { |b| public_add(a, b) }

    // Capturing a non-copy, non-drop resource strips both; intersected with a
    // private callee's copy+drop this leaves no abilities at all.
    public fun captures_resource(r: Res): |u64|u64 { |x| takes_res(r, x) }

    // Capturing a fully-abled struct cannot *add* abilities.
    public fun captures_plain(p: Plain): |u64|u64 has copy + drop { |x| takes_plain(p, x) }

    // Generic: `T: drop` resolves through the enclosing function's constraint
    // slice, so the closure is drop-only.
    public fun generic_captured<T: drop>(t: T): |T, bool|T {
        |other, flag| generic_pick(t, other, flag)
    }

    // Below, every callee is public, so the closure's abilities are exactly the
    // captured value's.
    public fun keep_vec_u64(v: vector<u64>, _y: u64): vector<u64> { v }
    public fun keep_vec_res(v: vector<Res>, _y: u64): vector<Res> { v }
    public fun keep_box_u64(b: Box<u64>, _y: u64): Box<u64> { b }
    public fun keep_box_res(b: Box<Res>, _y: u64): Box<Res> { b }
    public fun drop_box2(_b: Box2<Res>, y: u64): u64 { y }
    public fun drop_enum(_e: E, y: u64): u64 { y }
    public fun drop_signer(_s: signer, y: u64): u64 { y }

    // A vector is as able as its element.
    public fun captures_vec_u64(v: vector<u64>): |u64|vector<u64> has copy + drop + store {
        |y| keep_vec_u64(v, y)
    }
    public fun captures_vec_res(v: vector<Res>): |u64|vector<Res> has store {
        |y| keep_vec_res(v, y)
    }

    // A non-phantom argument predicates; `Box<Res>` keeps only store.
    public fun captures_box_u64(b: Box<u64>): |u64|Box<u64> has copy + drop + store {
        |y| keep_box_u64(b, y)
    }
    public fun captures_box_res(b: Box<Res>): |u64|Box<Res> has store {
        |y| keep_box_res(b, y)
    }

    // A phantom argument does not predicate, so `Box2<Res>` keeps everything.
    public fun captures_box2_res(b: Box2<Res>): |u64|u64 has copy + drop + store {
        |y| drop_box2(b, y)
    }

    // Enums resolve like structs.
    public fun captures_enum(e: E): |u64|u64 has copy + drop { |y| drop_enum(e, y) }

    // Signer is drop-only.
    public fun captures_signer(s: signer): |u64|u64 has drop { |y| drop_signer(s, y) }
}
