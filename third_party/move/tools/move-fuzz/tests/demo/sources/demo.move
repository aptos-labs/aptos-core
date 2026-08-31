// The demo project for `move-fuzz`.
//
// Every module below is annotated with the fuzzer capability it is meant to
// exercise. Nothing here is a realistic contract; it is deliberately small
// enough to read end to end while still covering the interesting paths of the
// pipeline: Phase 1 (single transaction), Phase 2 (multi transaction chains
// driven by the def-use graph), objects, generics, function values, and
// vector-shaped inputs.
//
// NOTE: this package must be built with `--language 2.4+`; see README.

/// Phase 1 warm-up: a single-transaction, state-free target.
///
/// Exercises: `vector<u8>` argument generation and the byte/string dictionary
/// in `mutate/mutator.rs`. Reaching `abort 42` needs nothing but a lucky (or
/// coverage-guided) input, so this module is the one that should light up
/// first. The unguarded index also lets the fuzzer discover an intrinsic
/// `VECTOR_OPERATION_ERROR` on short inputs.
module test::hello_fuzzer {
    public entry fun hello(input: vector<u8>) {
        if (input[0] == 0x48 /* h */) {
            if (input[1] == 0x65 /* e */) {
                if (input[2] == 0x6c /* l */) {
                    if (input[3] == 0x6c /* l */) {
                        if (input[4] == 0x6f /* o */) {
                            abort 42;
                        }
                    }
                }
            }
        }
    }
}

/// Phase 2 workhorse: an account-scoped ledger whose bugs are unreachable in
/// any single transaction.
///
/// Exercises:
/// - `MISSING_DATA`-driven chaining. Every entry point except `open` aborts
///   with `MISSING_DATA` until `open` has run for the same sender; that is the
///   signal `fuzzer.rs` collects into `missing_data_signals` and turns into
///   Phase 2 chains.
/// - Def-use edges over a resource: `open`/`deposit`/`withdraw` write
///   `test::vault::Vault`, `audit` reads it.
/// - `vector<u64>` state that grows across transactions, and a
///   `vector<address>` entry argument.
module test::vault {
    use std::signer;
    use std::vector;

    /// A vault already exists under this account.
    const E_ALREADY_OPEN: u64 = 1;
    /// Deposits must be non-zero.
    const E_ZERO_DEPOSIT: u64 = 2;
    /// The vault is frozen.
    const E_FROZEN: u64 = 3;
    /// Withdrawal exceeds the recorded balance.
    const E_INSUFFICIENT: u64 = 4;
    /// `balance + withdrawn` no longer matches the deposit history.
    const E_BROKEN_LEDGER: u64 = 5;

    /// Withdrawals of at least this much are charged a flat fee.
    const FEE_THRESHOLD: u64 = 100;
    /// The flat fee itself.
    const FEE: u64 = 1;

    struct Vault has key {
        balance: u64,
        withdrawn: u64,
        /// every deposit ever made, in order
        history: vector<u64>,
        frozen: bool,
    }

    /// Entry: the only function that works on a fresh account.
    ///
    /// Exercises: the head of every Phase 2 chain. Until this has run for a
    /// given sender, every other entry point in this module aborts before it
    /// executes a single interesting instruction.
    public entry fun open(owner: &signer) {
        let addr = signer::address_of(owner);
        assert!(!exists<Vault>(addr), E_ALREADY_OPEN);
        move_to(owner, Vault {
            balance: 0,
            withdrawn: 0,
            history: vector::empty<u64>(),
            frozen: false,
        });
    }

    /// Entry: record a deposit.
    ///
    /// Exercises: state that accumulates across transactions. The `u64`
    /// overflow on `balance` is only reachable after enough prior deposits,
    /// and `history` keeps growing so later reads get more expensive.
    public entry fun deposit(owner: &signer, amount: u64) {
        assert!(amount > 0, E_ZERO_DEPOSIT);
        let v = borrow_global_mut<Vault>(signer::address_of(owner));
        assert!(!v.frozen, E_FROZEN);
        v.balance = v.balance + amount;
        vector::push_back(&mut v.history, amount);
    }

    /// Entry: withdraw, charging a flat fee above `FEE_THRESHOLD`.
    ///
    /// Exercises: two planted, state-only defects.
    /// 1. the fee is subtracted *after* the sufficiency check, so withdrawing
    ///    the entire balance of a vault holding at least `FEE_THRESHOLD`
    ///    underflows (intrinsic `ARITHMETIC_ERROR`). Needs `open` + `deposit`
    ///    first, so it is out of reach for a one-shot fuzzer.
    /// 2. `withdrawn` never records the fee, which silently breaks the ledger
    ///    invariant that `audit` checks in a *later* transaction.
    public entry fun withdraw(owner: &signer, amount: u64) {
        let v = borrow_global_mut<Vault>(signer::address_of(owner));
        assert!(!v.frozen, E_FROZEN);
        assert!(amount <= v.balance, E_INSUFFICIENT);
        let fee = if (amount >= FEE_THRESHOLD) FEE else 0;
        v.balance = v.balance - amount - fee;
        v.withdrawn = v.withdrawn + amount;
    }

    /// Entry: close an empty vault.
    ///
    /// Exercises: the cheapest state-only failure in this package, and the
    /// shortest useful Phase 2 chain. `open` leaves `history` empty, so
    /// `open -> close` pops from an empty vector and aborts intrinsically with
    /// `VECTOR_OPERATION_ERROR`; neither transaction can do it alone.
    public entry fun close(owner: &signer) {
        let addr = signer::address_of(owner);
        let Vault { balance, withdrawn: _, history, frozen: _ } = move_from<Vault>(addr);
        let last = vector::pop_back(&mut history);
        assert!(balance >= last, E_INSUFFICIENT);
    }

    /// Entry: flip the freeze flag.
    ///
    /// Exercises: a boolean written by one transaction and read by another,
    /// which gives the def-use graph a cheap edge that gates `deposit` and
    /// `withdraw` without aborting on its own.
    public entry fun set_frozen(owner: &signer, frozen: bool) {
        let v = borrow_global_mut<Vault>(signer::address_of(owner));
        v.frozen = frozen;
    }

    /// Entry: check the ledger invariant of one account.
    ///
    /// Exercises: a signer-free entry point. The fuzzer has to guess the
    /// *address* of an account that already ran `open` (the address dictionary
    /// in `mutate/mutator.rs` supplies the simulated user accounts), and
    /// `E_BROKEN_LEDGER` can only fire after a fee-charging `withdraw` landed
    /// in an earlier transaction.
    public entry fun audit(owner_addr: address) {
        let v = borrow_global<Vault>(owner_addr);
        // widened to u128 on purpose: the sum must not overflow, otherwise the
        // interesting failure below would be masked by an arithmetic abort.
        let total = 0u128;
        let i = 0;
        let n = vector::length(&v.history);
        while (i < n) {
            total = total + (*vector::borrow(&v.history, i) as u128);
            i = i + 1;
        };
        assert!((v.balance as u128) + (v.withdrawn as u128) == total, E_BROKEN_LEDGER);
    }

    /// Entry: audit a batch.
    ///
    /// Exercises: `vector<address>` argument generation. Unknown addresses are
    /// skipped so a long random vector still makes progress instead of dying
    /// on its first element.
    public entry fun audit_many(owners: vector<address>) {
        let i = 0;
        let n = vector::length(&owners);
        while (i < n) {
            let owner = *vector::borrow(&owners, i);
            if (exists<Vault>(owner)) {
                audit(owner);
            };
            i = i + 1;
        };
    }

    /// Plain `public fun`, not an entry point.
    ///
    /// Exercises: `prep/function.rs` registers every `public` function, not
    /// just `entry` ones, so this getter becomes a fuzz target of its own and
    /// is also available to `prep/graph.rs` as a provider of `u64` values.
    public fun balance_of(owner_addr: address): u64 {
        borrow_global<Vault>(owner_addr).balance
    }
}

/// Objects: resources that live at their own address rather than under an
/// account.
///
/// Exercises:
/// - `Object<T>` as a fuzz input. `mint` writes `object::ObjectGroup`, which is
///   how `Mutator::update_object_dict` learns the address, and the `Badge`
///   write at that address is what binds the address to `Object<Badge>`.
/// - A generic entry function, so the fuzzer has to pick `T` out of the type
///   pool built by `fuzzer.rs::build_type_pool`.
/// - An invariant that only breaks after several transactions on the same
///   object.
module test::badge {
    use std::signer;
    use std::string::String;
    use aptos_framework::object::{Self, Object};

    /// The caller does not own this badge.
    const E_NOT_OWNER: u64 = 1;
    /// The badge is sealed and can no longer change.
    const E_SEALED: u64 = 2;
    /// The level went past `MAX_LEVEL`.
    const E_LEVEL_OVERFLOW: u64 = 3;

    /// Deliberately tiny so the bug below fits inside the default
    /// `--max-chain-length` of 5.
    const MAX_LEVEL: u64 = 1;

    struct Badge has key {
        name: String,
        level: u64,
        sealed: bool,
    }

    /// Entry: mint a named object carrying a `Badge`.
    ///
    /// Exercises: object creation. The address is
    /// `create_object_address(creator, seed)`, so distinct seeds give distinct
    /// objects and a repeated seed aborts inside the framework. This is the
    /// transaction every other entry point in this module depends on.
    public entry fun mint(creator: &signer, seed: vector<u8>, name: String) {
        let ctor = object::create_named_object(creator, seed);
        let obj_signer = object::generate_signer(&ctor);
        move_to(&obj_signer, Badge { name, level: 0, sealed: false });
    }

    /// Entry: bump the level of a badge you own.
    ///
    /// Exercises: `Object<Badge>` as a fuzz input plus an off-by-one that
    /// needs repetition. The bound is checked *before* the increment, so the
    /// level can be pushed to `MAX_LEVEL + 1`; only the call after that trips
    /// `E_LEVEL_OVERFLOW`.
    public entry fun level_up(caller: &signer, badge: Object<Badge>) {
        assert!(object::is_owner(badge, signer::address_of(caller)), E_NOT_OWNER);
        let b = borrow_global_mut<Badge>(object::object_address(&badge));
        assert!(!b.sealed, E_SEALED);
        assert!(b.level <= MAX_LEVEL, E_LEVEL_OVERFLOW);
        b.level = b.level + 1;
    }

    /// Entry: read-only invariant check.
    ///
    /// Exercises: the object counterpart of `test::vault::audit`. It can only
    /// fail once `level_up` has run `MAX_LEVEL + 1` times on the same object,
    /// which is a chain no single transaction can produce.
    public entry fun verify(badge: Object<Badge>) {
        let b = borrow_global<Badge>(object::object_address(&badge));
        assert!(b.level <= MAX_LEVEL, E_LEVEL_OVERFLOW);
    }

    /// Entry: freeze a badge forever.
    ///
    /// Exercises: a write that makes a previously reachable path unreachable,
    /// so the fuzzer has to care about ordering inside a chain.
    public entry fun seal(caller: &signer, badge: Object<Badge>) {
        assert!(object::is_owner(badge, signer::address_of(caller)), E_NOT_OWNER);
        let b = borrow_global_mut<Badge>(object::object_address(&badge));
        b.sealed = true;
    }

    /// Entry: generic rename, keyed off any object the caller owns.
    ///
    /// Exercises: type-argument selection. `Object<T>` is a *simple* input, so
    /// the fuzzer picks `T` from its type pool and an object address from the
    /// ones it has observed; the call only gets past `borrow_global_mut` when
    /// both happen to point at an address a previous `mint` created. Note that
    /// `prep/model.rs` expands this single function into one script per
    /// ability-set combination of `T` (eight of them for `T: key`), which is
    /// the cheapest place in this package to watch that behaviour.
    public entry fun rename_via<T: key>(caller: &signer, witness: Object<T>, name: String) {
        assert!(object::is_owner(witness, signer::address_of(caller)), E_NOT_OWNER);
        let b = borrow_global_mut<Badge>(object::object_address(&witness));
        assert!(!b.sealed, E_SEALED);
        b.name = name;
    }
}

/// Function values and provider chains.
///
/// Nothing here touches global storage: this module targets the *driver
/// generator* (`prep/model.rs`, `prep/graph.rs`, `prep/canvas.rs`) rather than
/// the fuzz loop. None of it is `entry`, and that is the point - function
/// values and plain structs are not valid transaction argument types, so these
/// functions are reachable only through a generated script.
module test::combinators {
    /// The folded value hit the poisoned constant.
    const E_POISONED: u64 = 1;

    /// A value the fuzzer cannot pass in directly, because a plain struct is
    /// not a whitelisted transaction argument. `prep/graph.rs` has to find its
    /// one provider (`recipe` below) and call it inside the driver.
    struct Recipe has drop {
        seed: u64,
        steps: u8,
    }

    /// Candidate #1 for any `|u64|u64` parameter.
    public fun double(x: u64): u64 {
        if (x > 0x7fffffffffffffff) x else x * 2
    }

    /// Candidate #2 for any `|u64|u64` parameter.
    public fun decrement(x: u64): u64 {
        if (x == 0) 0 else x - 1
    }

    /// Exercises: provider discovery. This is the only function in the project
    /// that returns a `Recipe`, so every driver generated for `brew` has to
    /// start by calling it.
    public fun recipe(seed: u64, steps: u8): Recipe {
        Recipe { seed, steps }
    }

    /// Exercises: function values, in the same driver as the provider chain.
    /// `prep/model.rs::find_matching_functions` enumerates every public
    /// `(u64) -> u64` in the project and emits one driver per candidate, so
    /// this single function fans out into several scripts, each carrying a
    /// line like `let v1 = |a0| test::combinators::double(a0);`.
    public fun brew(r: Recipe, f: |u64|u64 has copy + drop): u64 {
        let acc = r.seed;
        let i = 0;
        while (i < r.steps) {
            acc = f(acc);
            i = i + 1;
        };
        assert!(acc != 0xdeadbeef, E_POISONED);
        acc
    }
}
