// RUN: publish --print(bytecode,stackless,micro-ops)
module 0x1::test {
    use std::vector;

    struct Point has copy, drop {
        x: u64,
        y: u64,
    }

    struct Bag has copy, drop {
        id: u64,
        items: vector<u64>,
    }

    fun mk_bag(id: u64, a: u64, b: u64): Bag {
        let items = vector::empty<u64>();
        vector::push_back(&mut items, a);
        vector::push_back(&mut items, b);
        Bag { id, items }
    }

    fun bag_eq(id1: u64, a1: u64, b1: u64, id2: u64, a2: u64, b2: u64): bool {
        mk_bag(id1, a1, b1) == mk_bag(id2, a2, b2)
    }

    fun bag_neq(id1: u64, a1: u64, b1: u64, id2: u64, a2: u64, b2: u64): bool {
        mk_bag(id1, a1, b1) != mk_bag(id2, a2, b2)
    }

    fun mk_bag1(id: u64, a: u64): Bag {
        let items = vector::empty<u64>();
        vector::push_back(&mut items, a);
        Bag { id, items }
    }

    fun bag_eq_len(id1: u64, a1: u64, id2: u64, a2: u64, b2: u64): bool {
        mk_bag1(id1, a1) == mk_bag(id2, a2, b2)
    }

    fun mk_points(x1: u64, y1: u64, x2: u64, y2: u64): vector<Point> {
        let v = vector::empty<Point>();
        vector::push_back(&mut v, Point { x: x1, y: y1 });
        vector::push_back(&mut v, Point { x: x2, y: y2 });
        v
    }

    fun pts_eq(
        x1: u64, y1: u64, x2: u64, y2: u64,
        x3: u64, y3: u64, x4: u64, y4: u64,
    ): bool {
        mk_points(x1, y1, x2, y2) == mk_points(x3, y3, x4, y4)
    }

    fun pts_neq(
        x1: u64, y1: u64, x2: u64, y2: u64,
        x3: u64, y3: u64, x4: u64, y4: u64,
    ): bool {
        mk_points(x1, y1, x2, y2) != mk_points(x3, y3, x4, y4)
    }
}

// RUN: execute 0x1::test::bag_eq --args 1, 10, 20, 1, 10, 20
// CHECK: results: true

// RUN: execute 0x1::test::bag_eq --args 1, 10, 20, 2, 10, 20
// CHECK: results: false

// RUN: execute 0x1::test::bag_eq --args 1, 10, 20, 1, 10, 99
// CHECK: results: false

// RUN: execute 0x1::test::bag_neq --args 1, 10, 20, 1, 10, 20
// CHECK: results: false

// RUN: execute 0x1::test::bag_neq --args 1, 10, 20, 1, 99, 20
// CHECK: results: true

// RUN: execute 0x1::test::bag_neq --args 1, 10, 20, 7, 10, 20
// CHECK: results: true

// RUN: execute 0x1::test::bag_eq_len --args 1, 10, 1, 10, 20
// CHECK: results: false

// RUN: execute 0x1::test::pts_eq --args 1, 2, 3, 4, 1, 2, 3, 4
// CHECK: results: true

// RUN: execute 0x1::test::pts_eq --args 9, 2, 3, 4, 1, 2, 3, 4
// CHECK: results: false

// RUN: execute 0x1::test::pts_eq --args 1, 2, 3, 4, 1, 2, 3, 9
// CHECK: results: false

// RUN: execute 0x1::test::pts_neq --args 1, 2, 3, 4, 1, 2, 3, 4
// CHECK: results: false

// RUN: execute 0x1::test::pts_neq --args 1, 2, 3, 4, 1, 2, 3, 5
// CHECK: results: true
