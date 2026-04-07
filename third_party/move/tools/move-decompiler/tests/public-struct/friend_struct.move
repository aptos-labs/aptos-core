// Tests decompilation of friend-visibility structs and enums used across modules.
// The struct is declared with `friend` visibility in the defining module, which
// must also declare the consuming module as a friend. The binary stores friend
// struct visibility as Visibility::Friend; the decompiler emits `friend struct`.
// Based on compiler-v2 tests: structs_with_ability.move, enum_field_api.move

module 0x42::friend_defs {
    friend 0x42::friend_consumer;

    /// A friend struct: only accessible by declared friend modules.
    friend struct Config<T: copy + drop> has copy, drop {
        value: T,
        enabled: bool,
    }

    /// A friend enum: variants only accessible by declared friend modules.
    friend enum Status has copy, drop {
        Active { code: u64 },
        Inactive,
        Pending { code: u64, reason: u64 },
    }
}

module 0x42::friend_consumer {
    use 0x42::friend_defs::{Config, Status};

    // -----------------------------------------------------------------------
    // Config<T> — pack, unpack, field borrow, mutable field borrow
    // -----------------------------------------------------------------------

    fun make_config(value: u64, enabled: bool): Config<u64> {
        Config { value, enabled }
    }

    fun get_value(c: &Config<u64>): u64 {
        *&c.value
    }

    fun set_value(c: &mut Config<u64>, v: u64) {
        let r = &mut c.value;
        *r = v;
    }

    fun unpack_config(c: Config<u64>): (u64, bool) {
        let Config { value, enabled } = c;
        (value, enabled)
    }

    // -----------------------------------------------------------------------
    // Status — variant pack, variant test, variant unpack via match
    // -----------------------------------------------------------------------

    fun make_active(code: u64): Status {
        Status::Active { code }
    }

    fun make_inactive(): Status {
        Status::Inactive
    }

    fun is_active(s: &Status): bool {
        s is Status::Active
    }

    fun get_code(s: Status): u64 {
        match (s) {
            Status::Active { code } => code,
            Status::Pending { code, reason: _ } => code,
            Status::Inactive => 0,
        }
    }

    fun borrow_code(s: &Status): u64 {
        *&s.code
    }

    // -----------------------------------------------------------------------
    // End-to-end
    // -----------------------------------------------------------------------

    fun round_trip(): u64 {
        let c = make_config(42, true);
        set_value(&mut c, 99);
        let (v, _) = unpack_config(c);
        let s = make_active(7);
        let code = get_code(s);
        v + code
    }
}
