module 0xcafe::object {
    // Minimal stand-in for the framework's `object::ConstructorRef`.
    struct ConstructorRef has drop {}
}

module 0xcafe::ctor_ref_leaks {
    use 0xcafe::object::ConstructorRef;
    struct W<T> has drop { x: T }

    public fun error(r: ConstructorRef): ConstructorRef { r }
    public fun ret_ref(r: &ConstructorRef): &ConstructorRef { r }
    public fun ret_mut_ref(r: &mut ConstructorRef): &mut ConstructorRef { r }
    public fun ret_vec(v: vector<ConstructorRef>): vector<ConstructorRef> { v }
    public fun ret_tuple(r: ConstructorRef, u: u64): (u64, ConstructorRef) { (u, r) }
    public fun ret_wrapped(w: W<ConstructorRef>): W<ConstructorRef> { w }
    public fun ret_vecvec(v: vector<vector<ConstructorRef>>): vector<vector<ConstructorRef>> { v }
    public fun ret_tuple_ref(r: &ConstructorRef, u: u64): (u64, &ConstructorRef) { (u, r) }
}

#[lint::skip(return_constructor_ref)]
module 0xcafe::skip_module_ctor_ref_leak {
    use 0xcafe::object::ConstructorRef;

    public fun error(r: ConstructorRef): ConstructorRef { r }
}

module 0xcafe::skip_function_ctor_ref_leak {
    use 0xcafe::object::ConstructorRef;

    #[lint::skip(return_constructor_ref)]
    public fun error(r: ConstructorRef): ConstructorRef { r }
}
