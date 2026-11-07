// Function values as table values, as table keys, as BCS input and as tuple
// elements. Each instance must be emitted once: the Boogie name of a function
// type erases its abilities, and the source and bytecode forms of a type must
// agree on a single result (`bool`, not `(bool)`).
module 0x42::table_closure_value {
    use extensions::table::{Self, Table};
    use std::bcs;

    fun fun_keys(): Table<|u64| bool has store + copy + drop, u64> {
        table::new()
    }

    fun serialize_fun(f: |u64| bool has drop): vector<u8> {
        bcs::to_bytes(&f)
    }
    spec serialize_fun {
        ensures result == bcs::serialize(f);
    }

    // The same shapes with function types that differ only in abilities: one
    // table value instance, one key instance, one BCS instance.
    fun new_callbacks_store_only(): Table<address, |address, u64| bool has store> {
        table::new()
    }

    fun fun_keys_no_store(): Table<|u64| bool has copy + drop, u64> {
        table::new()
    }

    fun serialize_fun_copy(f: |u64| bool has copy + drop): vector<u8> {
        bcs::to_bytes(&f)
    }
    spec serialize_fun_copy {
        ensures result == bcs::serialize(f);
    }

    // Argument tuples that differ only in a nested closure's abilities are one
    // Boogie tuple type.
    #[persistent]
    fun positive(y: u64): bool { y > 0 }
    #[persistent]
    fun large(y: u64): bool { y > 1 }
    fun apply_copy(f: |u64, (|u64| bool has copy + drop)| bool has drop, x: u64): bool {
        f(x, positive)
    }
    fun apply_store(f: |u64, (|u64| bool has copy + drop + store)| bool has drop, x: u64): bool {
        f(x, large)
    }
    fun apply_both(x: u64): bool {
        apply_copy(|a, g| g(a), x) && apply_store(|a, g| g(a), x)
    }

    struct Registry has key {
        callbacks: Table<address, |address, u64| bool has store + copy + drop>
    }

    fun new_callbacks(): Table<address, |address, u64| bool has store + copy + drop> {
        table::new()
    }

    fun register(
        r: &mut Registry, k: address, f: |address, u64| bool has store + copy + drop
    ) {
        if (table::contains(&r.callbacks, k)) {
            table::remove(&mut r.callbacks, k);
        };
        table::add(&mut r.callbacks, k, f);
    }
    spec register {
        ensures table::spec_contains(r.callbacks, k);
    }

    fun is_registered(r: &Registry, k: address): bool {
        table::contains(&r.callbacks, k)
    }
    spec is_registered {
        ensures result == table::spec_contains(r.callbacks, k);
    }
}
