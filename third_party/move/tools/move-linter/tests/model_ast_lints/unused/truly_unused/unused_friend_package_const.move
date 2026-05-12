// Zero-user `friend` and `package` constants should be flagged as unused,
// mirroring how `unused_function` handles friend/package functions.
// `public` is not flagged — it may be a future external consumer's contract.

module 0xc0ffee::pkg {
    package const UNUSED_PKG: u64 = 1;        // SHOULD warn
    public const UNUSED_PUB: u64 = 2;         // should NOT warn (public)
}

module 0xc0ffee::friendly {
    friend 0xc0ffee::other;

    friend const UNUSED_FRIEND: u64 = 3;      // SHOULD warn
}

module 0xc0ffee::other {}
