module 0x42::m {
    use 0x1::vector;

    struct Option<Element> has copy, drop, store {
        vec: vector<Element>
    }

    public fun from_vec<Element>(vec: vector<Element>): Option<Element> {
        if (vector::length(&vec) > 1) abort(1);
        Option { vec }
    }
}
