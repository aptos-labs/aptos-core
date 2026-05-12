// Cross-module const reference compiles to a `const$NAME` accessor call,
// which is not allowed in a constant initializer.

module 0x42::M {
    public const A: u64 = 10;
    const B: u64 = A + 1; // allowed
}

module 0x42::N {
    use 0x42::M;

    public const B: u64 = M::A + 1; // rejected
}
