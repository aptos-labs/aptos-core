/// Specifications of the `Table` module.
spec aptos_std::table {

    // Make most of the public API intrinsic. Those functions have custom specifications in the prover.

    spec Table {
        pragma intrinsic;
    }

    spec new {
        pragma intrinsic;
    }

    spec destroy {
        pragma intrinsic;
    }

    spec add {
        pragma intrinsic;
    }

    spec borrow {
        pragma intrinsic;
    }

    spec borrow_mut {
        pragma intrinsic;
    }

    spec remove {
        pragma intrinsic;
    }

    spec contains {
        pragma intrinsic;
    }

    // Specification functions for tables

    spec native fun spec_table<K, V>(): Table<K, V>;
    spec native fun spec_len<K, V>(t: Table<K, V>): num;
    spec native fun spec_contains<K, V>(t: Table<K, V>, k: K): bool;
    spec native fun spec_add<K, V>(t: Table<K, V>, k: K, v: V): Table<K, V>;
    spec native fun spec_remove<K, V>(t: Table<K, V>, k: K): Table<K, V>;
    spec native fun spec_get<K, V>(t: Table<K, V>, k: K): V;
}
