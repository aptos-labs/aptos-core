// RUN: publish
module 0x42::generic_multi_inst_one_txn {
    struct Box<T> has copy, drop {
        value: T,
    }

    fun make_box<T>(v: T): Box<T> {
        Box { value: v }
    }

    fun get<T: copy>(b: &Box<T>): T {
        b.value
    }

    fun run(v: u64): u64 {
        let a = make_box(v);
        let b = make_box((v as u8));
        let c = make_box(true);
        let d = make_box(@0xabc);
        let total = get(&a) + (get(&b) as u64);
        let total = total + (if (get(&c)) { 1 } else { 0 });
        let total = total + (if (get(&d) == @0xabc) { 10 } else { 0 });
        total
    }

    fun repeated(v: u64): u64 {
        let total = 0;
        let i = 0;
        while (i < 5) {
            total = total + get(&make_box(v + i));
            i = i + 1;
        };
        total
    }
}

// RUN: execute 0x42::generic_multi_inst_one_txn::run --args 200
// CHECK: results: 411

// RUN: execute 0x42::generic_multi_inst_one_txn::repeated --args 10
// CHECK: results: 60
