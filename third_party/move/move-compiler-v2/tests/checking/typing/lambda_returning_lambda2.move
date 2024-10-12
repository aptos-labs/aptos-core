module 0x8675309::M {

    public inline fun foreach_caller2<T>(_v: &vector<T>, _action: ||(|&T|)) {
        abort(1)
    }

    public fun whacky_foreach2() {
        let v = vector[1, 2, 3];
        let sum = 0;
        foreach_caller2(&v, ||(|e| sum = sum + *e)) // expected to be not implemented
    }
}
