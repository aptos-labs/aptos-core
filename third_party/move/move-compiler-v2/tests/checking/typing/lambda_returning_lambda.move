module 0x8675309::M {
    use 0x1::XVector;

    public inline fun foreach<T>(v: &vector<T>, action: |&T|) { // expected to be not implemented
        let i = 0;
        while (i < XVector::length(v)) {
            action(XVector::borrow(v, i));
            i = i + 1;
        }
    }

    public inline fun foreach_caller<T>(v: &vector<T>, action: ||(|&T|)) {
        foreach<T>(v, action())
    }

    public fun whacky_foreach() {
        let v = vector[1, 2, 3];
        let sum = 0;
        foreach_caller(&v, ||(|e| sum = sum + *e)) // expected to be not implemented
    }
}

module 0x1::XVector {
    public fun length<T>(_v: &vector<T>): u64 { abort(1) }
    public fun is_empty<T>(_v: &vector<T>): bool { abort(1) }
    public fun borrow<T>(_v: &vector<T>, _i: u64): &T { abort(1) }
    public fun pop_back<T>(_v: &mut vector<T>): T { abort(1) }
}
