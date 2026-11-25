// Equality that differs only in the last field of the last record, three containers
// deep: vector<Node> -> Node::Branch -> vector<Rec>. Values are built independently.

// RUN: publish
module 0x66::late_neq {
    struct Rec has copy, drop { a: u8, b: u64, c: u16 }

    enum Node has copy, drop { Leaf { n: u64 }, Branch { recs: vector<Rec> } }

    fun build(last: u16): vector<Node> {
        vector[
            Node::Leaf { n: 1 },
            Node::Branch { recs: vector[Rec { a: 1, b: 2, c: 3 }, Rec { a: 4, b: 5, c: last }] },
        ]
    }

    public fun compare(last: u16, other_last: u16): (bool, bool, bool) {
        let left = build(last);
        let left_copy = build(last);
        let right = build(other_last);
        (left == left_copy, left == right, right == left)
    }

    public fun compare_neq(last: u16, other_last: u16): (bool, bool) {
        let left = build(last);
        let right = build(other_last);
        (left != right, right != left)
    }
}

// RUN: execute 0x66::late_neq::compare --args 6, 7
// CHECK: results: true, false, false

// RUN: execute 0x66::late_neq::compare --args 6, 6
// CHECK: results: true, true, true

// RUN: execute 0x66::late_neq::compare_neq --args 6, 7
// CHECK: results: true, true

// RUN: execute 0x66::late_neq::compare_neq --args 6, 6
// CHECK: results: false, false
