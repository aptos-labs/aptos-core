// `friend`/`package` constants with no cross-module users → flagged.
// `public` and `private` constants → not flagged.

module 0xc0ffee::pkg {
    package const PKG_ONLY_LOCAL: u64 = 30;       // flagged
    package const PKG_CROSS: u64 = 40;            // ok: used by consumer
    const PRIV: u64 = 70;
    public const PUB_ONLY_LOCAL: u64 = 10;        // not flagged (public)

    public fun read_all(): u64 {
        PKG_ONLY_LOCAL + PKG_CROSS + PRIV + PUB_ONLY_LOCAL
    }
}

module 0xc0ffee::friendly {
    friend 0xc0ffee::consumer;

    friend const FRIEND_ONLY_LOCAL: u64 = 50;     // flagged
    friend const FRIEND_CROSS: u64 = 60;          // ok: used by friend

    public fun read_all(): u64 { FRIEND_ONLY_LOCAL + FRIEND_CROSS }
}

module 0xc0ffee::consumer {
    use 0xc0ffee::pkg;
    use 0xc0ffee::friendly;

    public fun use_pkg(): u64 { pkg::PKG_CROSS }
    public fun use_friend(): u64 { friendly::FRIEND_CROSS }
}

// Friend visibility on a module with no `friend` declarations → flagged.
module 0xc0ffee::orphan {
    friend const ORPHAN_FRIEND: u64 = 1;          // flagged

    public fun read(): u64 { ORPHAN_FRIEND }
}
