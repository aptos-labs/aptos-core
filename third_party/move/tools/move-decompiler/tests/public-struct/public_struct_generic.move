// Tests decompilation of public generic structs and enums used across modules.
// Exercises: pack/unpack of generic types, field borrow on generic struct,
// variant test/pack/unpack on generic enum, and nested generic instantiations.

module 0x42::generic_defs {

    /// A generic key-value pair.
    public struct Pair<K: copy + drop, V: copy + drop> has copy, drop {
        key: K,
        value: V,
    }

    /// A generic wrapper that adds a tag to any value.
    public struct Tagged<T: copy + drop> has copy, drop {
        tag: u64,
        inner: T,
    }

    /// A Result-like generic enum.
    public enum Outcome<T: copy + drop, E: copy + drop> has copy, drop {
        Ok { value: T },
        Err { error: E },
    }

    /// A generic Option-like enum for nullable values.
    public enum Maybe<T: copy + drop> has copy, drop {
        None,
        Some { val: T },
    }
}

module 0x42::generic_consumer {
    use 0x42::generic_defs::{Pair, Tagged, Outcome, Maybe};

    // -----------------------------------------------------------------------
    // Pair<K, V> — pack, unpack, field borrow
    // -----------------------------------------------------------------------

    fun make_pair(k: u64, v: bool): Pair<u64, bool> {
        Pair { key: k, value: v }
    }

    fun get_key(p: &Pair<u64, bool>): u64 {
        *&p.key
    }

    fun swap_pair(p: Pair<u64, bool>): Pair<bool, u64> {
        let Pair { key: k, value: v } = p;
        Pair { key: v, value: k }
    }

    // -----------------------------------------------------------------------
    // Tagged<T> — pack, unpack, nested generic (Tagged<Pair<u64, u64>>)
    // -----------------------------------------------------------------------

    fun tag_value(tag: u64, inner: u64): Tagged<u64> {
        Tagged { tag, inner }
    }

    fun untag(t: Tagged<u64>): (u64, u64) {
        let Tagged { tag, inner } = t;
        (tag, inner)
    }

    fun tag_pair(tag: u64, k: u64, v: u64): Tagged<Pair<u64, u64>> {
        Tagged { tag, inner: Pair { key: k, value: v } }
    }

    fun get_tagged_key(t: &Tagged<Pair<u64, u64>>): u64 {
        *&t.inner.key
    }

    // -----------------------------------------------------------------------
    // Outcome<T, E> — variant pack, variant test, variant unpack via match
    // -----------------------------------------------------------------------

    fun make_ok(v: u64): Outcome<u64, u8> {
        Outcome::Ok { value: v }
    }

    fun make_err(e: u8): Outcome<u64, u8> {
        Outcome::Err { error: e }
    }

    fun is_ok(o: &Outcome<u64, u8>): bool {
        o is Outcome::Ok
    }

    fun unwrap_or(o: Outcome<u64, u8>, default: u64): u64 {
        match (o) {
            Outcome::Ok { value } => value,
            Outcome::Err { error: _ } => default,
        }
    }

    fun map_ok(o: Outcome<u64, u8>, addend: u64): Outcome<u64, u8> {
        match (o) {
            Outcome::Ok { value } => Outcome::Ok { value: value + addend },
            Outcome::Err { error } => Outcome::Err { error },
        }
    }

    // -----------------------------------------------------------------------
    // Maybe<T> — None variant (no fields), Some variant, nested Maybe<Pair>
    // -----------------------------------------------------------------------

    fun none_u64(): Maybe<u64> {
        Maybe::None
    }

    fun some_u64(v: u64): Maybe<u64> {
        Maybe::Some { val: v }
    }

    fun is_some(m: &Maybe<u64>): bool {
        m is Maybe::Some
    }

    fun unwrap_maybe(m: Maybe<u64>): u64 {
        match (m) {
            Maybe::Some { val } => val,
            Maybe::None => 0,
        }
    }

    fun wrap_pair(k: u64, v: u64): Maybe<Pair<u64, u64>> {
        Maybe::Some { val: Pair { key: k, value: v } }
    }

    // -----------------------------------------------------------------------
    // Combined: Outcome<Tagged<u64>, u8>
    // -----------------------------------------------------------------------

    fun tagged_ok(tag: u64, inner: u64): Outcome<Tagged<u64>, u8> {
        Outcome::Ok { value: Tagged { tag, inner } }
    }

    fun extract_inner(o: Outcome<Tagged<u64>, u8>): u64 {
        match (o) {
            Outcome::Ok { value: Tagged { tag: _, inner } } => inner,
            Outcome::Err { error: _ } => 0,
        }
    }

    // -----------------------------------------------------------------------
    // End-to-end exercise
    // -----------------------------------------------------------------------

    fun end_to_end(): u64 {
        let p = make_pair(10, true);
        let k = get_key(&p);
        let swapped = swap_pair(p);
        let Pair { key: _, value: orig_k } = swapped;

        let t = tag_pair(99, 3, 7);
        let inner_k = get_tagged_key(&t);

        let ok = make_ok(42);
        let mapped = map_ok(ok, 8);
        let result = unwrap_or(mapped, 0);

        let m = some_u64(5);
        let mv = unwrap_maybe(m);

        k + orig_k + inner_k + result + mv
    }
}
