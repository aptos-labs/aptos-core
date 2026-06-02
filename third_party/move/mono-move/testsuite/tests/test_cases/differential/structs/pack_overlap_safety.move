// RUN: publish --print(stackless,micro-ops)
module 0xc0ffee::pack_overlap_safety {
    struct Pair has drop { a: u64, b: u64 }

    fun split(): (u64, u64) {
        (10, 20)
    }

    fun consume(p: Pair): u64 {
        p.a * 1000 + p.b
    }

    fun pair_provider(): Pair {
        Pair { a: 30, b: 40 }
    }

    fun consume_two(first: u64, second: u64): u64 {
        first * 1000 + second
    }

    public fun pack_natural(): u64 {
        let (x, y) = split();
        consume(Pair { a: x, b: y })
    }

    public fun pack_reordered(): u64 {
        let (x, y) = split();
        consume(Pair { a: y, b: x })
    }

    public fun unpack_natural(): u64 {
        let Pair { a, b } = pair_provider();
        consume_two(a, b)
    }

    public fun unpack_reordered(): u64 {
        let Pair { a, b } = pair_provider();
        consume_two(b, a)
    }
}

// RUN: execute 0xc0ffee::pack_overlap_safety::pack_natural
// CHECK: results: 10020

// RUN: execute 0xc0ffee::pack_overlap_safety::pack_reordered
// CHECK: results: 20010

// RUN: execute 0xc0ffee::pack_overlap_safety::unpack_natural
// CHECK: results: 30040

// RUN: execute 0xc0ffee::pack_overlap_safety::unpack_reordered
// CHECK: results: 40030
