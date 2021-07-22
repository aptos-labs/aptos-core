// dep: ../../move-stdlib/sources/Vector.move

module 0x1::VecEq {
    use Std::Vector;

    struct G { v: vector<u64> }

    spec G {
        invariant v == vec<u64>(10);
    }

    public fun new(): G {
        G { v: Vector::singleton(10) }
    }

    // should complain
    public fun leak_v(g: &mut G): &mut vector<u64> {
        &mut g.v
    }

    // should also complain
    public fun leak_index_into_v(g: &mut G): &mut u64 {
        Vector::borrow_mut(&mut g.v, 0)
    }
}
