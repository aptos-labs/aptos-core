module 0xcafe::signer_leaks {
    struct W<T> has drop { x: T }

    public fun error(s: signer): signer { s }
    public fun ret_ref(s: &signer): &signer { s }
    public fun ret_mut_ref(s: &mut signer): &mut signer { s }
    public fun ret_vec(v: vector<signer>): vector<signer> { v }
    public fun ret_tuple(s: signer, t: u64): (u64, signer) { (t, s) }
    public fun ret_wrapped(w: W<signer>): W<signer> { w }
}

#[lint::skip(return_signer)]
module 0xcafe::skip_module_leak {
    public fun error(s: signer): signer { s }
}

module 0xcafe::skip_function_leak {
    #[lint::skip(return_signer)]
    public fun error(s: signer): signer { s }
}
