module 0x42::M {

    struct CoolStruct has drop {}

    // script functions with non-invocable signatures

    public(script) fun signer_ref(_: &signer) {}

    public(script) fun late_signer(_u: u64, _s: signer) {}

    public(script) fun struct_arg(_: CoolStruct) {}

    public(script) fun u64_ret(): u64 {
        0
    }

    public(script) fun struct_ret(): CoolStruct {
        CoolStruct {}
    }

}
