// RUN: publish
module 0x42::generic_through_calls {
    struct Box<T> has copy, drop {
        value: T,
    }

    fun make_box<T>(v: T): Box<T> {
        Box { value: v }
    }

    fun get<T: copy>(b: &Box<T>): T {
        b.value
    }

    fun wrap_twice<T>(v: T): Box<Box<T>> {
        make_box(make_box(v))
    }

    struct Wide<T> has copy, drop {
        value: T,
        tag: u64,
    }

    fun wrap_wide<T>(v: T): Box<Wide<T>> {
        make_box(Wide { value: v, tag: 7 })
    }

    fun composed_wide(v: u64): u64 {
        let b = wrap_wide(v);
        let Box { value: wide } = b;
        let Wide { value, tag } = wide;
        value + tag
    }

    fun direct(v: u64): u64 {
        let b = make_box(v);
        get(&b)
    }

    fun composed(v: u64): u64 {
        let bb = wrap_twice(v);
        let inner = get(&bb);
        get(&inner)
    }

    fun unwrap_composed(v: u64): u64 {
        let bb = wrap_twice(v);
        let Box { value: inner } = bb;
        let Box { value } = inner;
        value
    }
}

// RUN: execute 0x42::generic_through_calls::direct --args 42
// CHECK: results: 42

// RUN: execute 0x42::generic_through_calls::composed --args 123456
// CHECK: results: 123456

// RUN: execute 0x42::generic_through_calls::unwrap_composed --args 987
// CHECK: results: 987

// RUN: execute 0x42::generic_through_calls::composed_wide --args 600
// CHECK: results: 607
