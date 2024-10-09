#[mutation::skip]
module 0x41::DoNotMutateThisModule {
    public fun sum(a: u32, b: u32): u32 {
        a + b
    }
}

module 0x42::MutatingAllowed {
    public fun sum(a: u32, b: u32): u32 {
        a + b
    }

    #[mutation::skip]
    public fun sum_no_mutation(a: u32, b: u32): u32 {
        a + b
    }
}
