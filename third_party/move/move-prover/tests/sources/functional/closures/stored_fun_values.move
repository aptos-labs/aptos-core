// Copyright © Aptos Foundation
// Tests for stored function values in structs with behavioral predicate invariants.
// Verifies that the prover can:
// 1. Check struct invariants at pack time against concrete closures
// 2. Assume struct invariants at borrow time via StructFieldInfo variants
// 3. Use assumed invariants when calling stored function values
// 4. Handle ensures_of and result_of on struct field functions
// 5. Handle stored functions that read/modify global resources

module 0x42::stored_fun_values {

    // =========================================================================
    // 1. Basic: aborts_of invariant on pure functions
    // =========================================================================

    struct Transformer has key, drop {
        f: |u64|u64 has copy+store+drop,
    }
    spec Transformer {
        invariant forall x: u64: !aborts_of<f>(x);
    }

    /// Helper: a safe function that never aborts (identity)
    public fun identity(x: u64): u64 {
        x
    }
    spec identity {
        aborts_if false;
        ensures result == x;
    }

    /// Helper: a function that aborts on overflow
    public fun add_one(x: u64): u64 {
        x + 1
    }
    spec add_one {
        aborts_if x + 1 > MAX_U64;
        ensures result == x + 1;
    }

    /// Pack with a non-aborting function: should verify
    fun create_valid(): Transformer {
        Transformer { f: identity }
    }

    /// Pack with a potentially aborting function: should fail data invariant
    fun create_invalid(): Transformer {
        Transformer { f: add_one } // error: data invariant does not hold
    }

    /// Call a stored function value from a resource — abort-free by invariant
    fun use_transformer(addr: address): u64 {
        let t = &Transformer[addr];
        (t.f)(42)
    }
    spec use_transformer {
        requires exists<Transformer>(addr);
        // The function in the transformer never aborts (from the invariant),
        // so this function should not abort either (besides the exists check).
        aborts_if !exists<Transformer>(addr);
    }

    // =========================================================================
    // 2. ensures_of invariant — constraining the result
    // =========================================================================

    struct Monotone has key, drop {
        f: |u64|u64 has copy+store+drop,
    }
    spec Monotone {
        invariant forall x: u64: !aborts_of<f>(x);
        // The stored function always returns a value >= its input
        invariant forall x: u64, r: u64: ensures_of<f>(x, r) ==> r >= x;
    }

    /// Pack with identity (returns x >= x): should verify
    fun create_monotone_valid(): Monotone {
        Monotone { f: identity }
    }

    /// Helper: a function that returns x - 1 (violates monotonicity for x > 0)
    public fun sub_one(x: u64): u64 {
        if (x > 0) { x - 1 } else { 0 }
    }
    spec sub_one {
        aborts_if false;
        ensures x > 0 ==> result == x - 1;
        ensures x == 0 ==> result == 0;
    }

    /// Pack with sub_one: should fail monotonicity invariant
    fun create_monotone_invalid(): Monotone {
        Monotone { f: sub_one } // error: data invariant does not hold
    }

    /// Use the monotone function — result is at least the input
    fun use_monotone(addr: address, x: u64): u64 {
        let m = &Monotone[addr];
        (m.f)(x)
    }
    spec use_monotone {
        requires exists<Monotone>(addr);
        aborts_if !exists<Monotone>(addr);
        ensures result >= x;
    }

    // =========================================================================
    // 3. result_of on struct fields — reasoning about concrete values
    // =========================================================================

    struct Doubler has key, drop {
        f: |u64|u64 has copy+store+drop,
    }
    spec Doubler {
        invariant forall x: u64: !aborts_of<f>(x);
        invariant forall x: u64: result_of<f>(x) == 2 * x;
    }

    public fun double(x: u64): u64 {
        x * 2
    }
    spec double {
        aborts_if x * 2 > MAX_U64;
        ensures result == 2 * x;
    }

    /// This should fail: double can abort on large inputs
    fun create_doubler_unsafe(): Doubler {
        Doubler { f: double } // error: data invariant does not hold (aborts_of)
    }

    // =========================================================================
    // 4. Stored functions that read resources
    // =========================================================================

    struct Counter has key {
        value: u64,
    }

    struct Reader has key, drop {
        f: |address|u64 has copy+store+drop,
    }
    spec Reader {
        // The stored reader function never aborts (assumes resource exists)
        invariant forall a: address: !aborts_of<f>(a);
    }

    // A function that reads a global resource — can abort when missing
    #[persistent]
    fun read_counter(addr: address): u64 {
        Counter[addr].value
    }
    spec read_counter {
        pragma opaque;
        aborts_if !exists<Counter>(addr);
        ensures result == Counter[addr].value;
    }

    /// Pack with read_counter: should fail because read_counter can abort
    fun create_reader_invalid(): Reader {
        Reader { f: read_counter } // error: data invariant does not hold
    }

    // A safe reader that guards resource access — never aborts
    #[persistent]
    fun safe_read_counter(addr: address): u64 {
        if (exists<Counter>(addr)) { Counter[addr].value } else { 0 }
    }
    spec safe_read_counter {
        pragma opaque;
        aborts_if false;
        ensures exists<Counter>(addr) ==> result == Counter[addr].value;
        ensures !exists<Counter>(addr) ==> result == 0;
    }

    /// Pack with safe_read_counter: should verify (never aborts)
    fun create_reader_valid(): Reader {
        Reader { f: safe_read_counter }
    }

    /// Call the stored reader through a resource — abort-free by invariant
    fun use_reader(reader_addr: address, target_addr: address): u64 {
        let r = &Reader[reader_addr];
        (r.f)(target_addr)
    }
    spec use_reader {
        requires exists<Reader>(reader_addr);
        aborts_if !exists<Reader>(reader_addr);
    }

    // =========================================================================
    // 5. Stored functions that modify resources
    // =========================================================================

    struct Modifier has key, drop {
        f: |&signer| has copy+store+drop,
    }
    spec Modifier {
        invariant forall s: signer: !aborts_of<f>(s);
    }

    // A function that modifies a global resource — can abort
    #[persistent]
    fun increment_counter(s: &signer) {
        let addr = std::signer::address_of(s);
        Counter[addr].value = Counter[addr].value + 1;
    }
    spec increment_counter {
        pragma opaque;
        modifies Counter[std::signer::address_of(s)];
        aborts_if !exists<Counter>(std::signer::address_of(s));
        aborts_if Counter[std::signer::address_of(s)].value + 1 > MAX_U64;
        ensures Counter[std::signer::address_of(s)].value ==
            old(Counter[std::signer::address_of(s)].value) + 1;
    }

    /// Pack with increment_counter: should fail because it can abort
    fun create_modifier_invalid(): Modifier {
        Modifier { f: increment_counter } // error: data invariant does not hold
    }

    // A safe modifier that guards resource access — never aborts
    #[persistent]
    fun safe_increment(s: &signer) {
        let addr = std::signer::address_of(s);
        if (exists<Counter>(addr) && Counter[addr].value < 18446744073709551615) {
            Counter[addr].value = Counter[addr].value + 1;
        }
    }
    spec safe_increment {
        pragma opaque;
        modifies Counter[std::signer::address_of(s)];
        aborts_if false;
        ensures exists<Counter>(std::signer::address_of(s))
            && old(Counter[std::signer::address_of(s)].value) < MAX_U64
            ==> Counter[std::signer::address_of(s)].value ==
                old(Counter[std::signer::address_of(s)].value) + 1;
    }

    /// Pack with safe_increment: should verify (never aborts)
    fun create_modifier_valid(): Modifier {
        Modifier { f: safe_increment }
    }

    /// Call the stored modifier through a resource — abort-free by invariant
    fun use_modifier(modifier_addr: address, s: &signer) {
        let m = &Modifier[modifier_addr];
        (m.f)(s)
    }
    spec use_modifier {
        requires exists<Modifier>(modifier_addr);
        aborts_if !exists<Modifier>(modifier_addr);
    }

    // =========================================================================
    // 6. Combined: struct invariant constrains both abort and result
    // =========================================================================

    struct SafeOp has key, drop {
        f: |u64, u64|u64 has copy+store+drop,
    }
    spec SafeOp {
        invariant forall x: u64, y: u64: !aborts_of<f>(x, y);
        invariant forall x: u64, y: u64, r: u64:
            ensures_of<f>(x, y, r) ==> r <= x + y;
    }

    /// Helper: saturating add (never aborts, result <= x + y)
    public fun saturating_add(x: u64, y: u64): u64 {
        let sum = (x as u128) + (y as u128);
        if (sum > (18446744073709551615u128)) {
            18446744073709551615u64
        } else {
            (sum as u64)
        }
    }
    spec saturating_add {
        aborts_if false;
        ensures result <= x + y;
    }

    /// Pack with saturating_add: should verify both invariants
    fun create_safe_op(): SafeOp {
        SafeOp { f: saturating_add }
    }

    /// Use the safe operation — no abort, bounded result
    fun use_safe_op(addr: address, a: u64, b: u64): u64 {
        let op = &SafeOp[addr];
        (op.f)(a, b)
    }
    spec use_safe_op {
        requires exists<SafeOp>(addr);
        aborts_if !exists<SafeOp>(addr);
        ensures result <= a + b;
    }

    // =========================================================================
    // 7. reads_of in struct spec: declare resource read access
    // =========================================================================

    struct Config has key {
        active: bool,
    }

    struct CounterReader has key, drop {
        f: |address|u64 has copy+store+drop,
    }
    spec CounterReader {
        reads_of<f> Counter;
        invariant forall a: address: !aborts_of<f>(a);
    }

    /// Pack with safe_read_counter (reads Counter only): should verify
    fun create_counter_reader(): CounterReader {
        CounterReader { f: safe_read_counter }
    }

    /// Call through resource — abort-free, reads Counter.
    /// The reads_of<f> Counter declaration means Counter is preserved after the call.
    fun use_counter_reader(reader_addr: address, target: address): u64 {
        let r = &CounterReader[reader_addr];
        (r.f)(target)
    }
    spec use_counter_reader {
        requires exists<CounterReader>(reader_addr);
        aborts_if !exists<CounterReader>(reader_addr);
        // Counter is unchanged because reads_of<f> declares read-only access
        ensures Counter[target] == old(Counter[target]);
        // Config is also unchanged (not in reads_of, so pure = preserved)
        ensures Config[target] == old(Config[target]);
    }

    // =========================================================================
    // 8. modifies_of in struct spec: declare resource write access
    // =========================================================================

    struct CounterModifier has key, drop {
        f: |&signer| has copy+store+drop,
    }
    spec CounterModifier {
        modifies_of<f>(s: signer) Counter[std::signer::address_of(s)];
        invariant forall s: signer: !aborts_of<f>(s);
    }

    /// Pack with safe_increment (modifies Counter): should verify
    fun create_counter_modifier(): CounterModifier {
        CounterModifier { f: safe_increment }
    }

    /// Call through resource — abort-free, modifies Counter at signer's address.
    /// Config is NOT in modifies_of, so it must be preserved.
    fun use_counter_modifier(modifier_addr: address, s: &signer) {
        let m = &CounterModifier[modifier_addr];
        (m.f)(s)
    }
    spec use_counter_modifier {
        requires exists<CounterModifier>(modifier_addr);
        aborts_if !exists<CounterModifier>(modifier_addr);
        // Config is unchanged — not in modifies_of, so preserved by frame condition
        ensures Config[std::signer::address_of(s)] == old(Config[std::signer::address_of(s)]);
    }

    // =========================================================================
    // 9. reads_of + modifies_of combined on same struct
    // =========================================================================

    struct ConfigAwareModifier has key, drop {
        f: |&signer| has copy+store+drop,
    }
    spec ConfigAwareModifier {
        reads_of<f> Config;
        modifies_of<f>(s: signer) Counter[std::signer::address_of(s)];
        invariant forall s: signer: !aborts_of<f>(s);
    }

    // A function that reads Config and modifies Counter (never aborts)
    #[persistent]
    fun config_aware_increment(s: &signer) {
        let addr = std::signer::address_of(s);
        if (exists<Config>(addr) && Config[addr].active
            && exists<Counter>(addr) && Counter[addr].value < 18446744073709551615)
        {
            Counter[addr].value = Counter[addr].value + 1;
        }
    }
    spec config_aware_increment {
        pragma opaque;
        modifies Counter[std::signer::address_of(s)];
        aborts_if false;
    }

    /// Pack with config_aware_increment: should verify
    fun create_config_aware_modifier(): ConfigAwareModifier {
        ConfigAwareModifier { f: config_aware_increment }
    }

    /// Call through resource — Config preserved, Counter may change
    fun use_config_aware_modifier(modifier_addr: address, s: &signer) {
        let m = &ConfigAwareModifier[modifier_addr];
        (m.f)(s)
    }
    spec use_config_aware_modifier {
        requires exists<ConfigAwareModifier>(modifier_addr);
        aborts_if !exists<ConfigAwareModifier>(modifier_addr);
        // Config declared as reads_of — must be preserved
        ensures Config[std::signer::address_of(s)] == old(Config[std::signer::address_of(s)]);
    }

    // =========================================================================
    // 10. Negative: wrong ensures about modifiable resource should fail
    // =========================================================================

    /// Claiming Counter is unchanged when modifies_of<f> declares it writable: should fail
    fun use_counter_modifier_wrong(modifier_addr: address, s: &signer) {
        let m = &CounterModifier[modifier_addr];
        (m.f)(s)
    }
    spec use_counter_modifier_wrong {
        requires exists<CounterModifier>(modifier_addr);
        aborts_if !exists<CounterModifier>(modifier_addr);
        ensures Counter[std::signer::address_of(s)]
            == old(Counter[std::signer::address_of(s)]); // error: post-condition does not hold
    }

    // =========================================================================
    // 11. reads_of<f> * — wildcard read access
    // =========================================================================

    struct AnyReader has key, drop {
        f: |address|u64 has copy+store+drop,
    }
    spec AnyReader {
        reads_of<f> *;
        invariant forall a: address: !aborts_of<f>(a);
    }

    /// Pack with safe_read_counter (reads Counter): compliant with wildcard
    fun create_any_reader(): AnyReader {
        AnyReader { f: safe_read_counter }
    }

    // A function that reads both Counter and Config — tests wildcard accepts
    // functions accessing multiple specific resources
    #[persistent]
    fun safe_read_both(addr: address): u64 {
        if (exists<Config>(addr) && Config[addr].active && exists<Counter>(addr)) {
            Counter[addr].value
        } else {
            0
        }
    }
    spec safe_read_both {
        pragma opaque;
        aborts_if false;
    }

    /// Pack with safe_read_both (reads Counter AND Config): wildcard accepts it
    fun create_any_reader_multi(): AnyReader {
        AnyReader { f: safe_read_both }
    }

    /// Call through resource — reads_of * preserves all memory
    fun use_any_reader(reader_addr: address, target: address): u64 {
        let r = &AnyReader[reader_addr];
        (r.f)(target)
    }
    spec use_any_reader {
        requires exists<AnyReader>(reader_addr);
        aborts_if !exists<AnyReader>(reader_addr);
        // reads_of * preserves all memory
        ensures Counter[target] == old(Counter[target]);
        ensures Config[target] == old(Config[target]);
    }

    // =========================================================================
    // 12. modifies_of<f> * — wildcard write access
    // =========================================================================

    struct AnyModifier has key, drop {
        f: |&signer| has copy+store+drop,
    }
    spec AnyModifier {
        modifies_of<f> *;
        invariant forall s: signer: !aborts_of<f>(s);
    }

    /// Pack with safe_increment (modifies Counter): compliant with wildcard
    fun create_any_modifier(): AnyModifier {
        AnyModifier { f: safe_increment }
    }

    /// Pack with config_aware_increment (reads Config, modifies Counter):
    /// wildcard accepts functions touching multiple resources
    fun create_any_modifier_multi(): AnyModifier {
        AnyModifier { f: config_aware_increment }
    }

    /// Call through resource — modifies_of * means nothing is preserved
    fun use_any_modifier(modifier_addr: address, s: &signer) {
        let m = &AnyModifier[modifier_addr];
        (m.f)(s)
    }
    spec use_any_modifier {
        requires exists<AnyModifier>(modifier_addr);
        aborts_if !exists<AnyModifier>(modifier_addr);
    }

    /// Claiming Counter unchanged with modifies_of * should fail
    fun use_any_modifier_wrong(modifier_addr: address, s: &signer) {
        let m = &AnyModifier[modifier_addr];
        (m.f)(s)
    }
    spec use_any_modifier_wrong {
        requires exists<AnyModifier>(modifier_addr);
        aborts_if !exists<AnyModifier>(modifier_addr);
        ensures Counter[std::signer::address_of(s)]
            == old(Counter[std::signer::address_of(s)]); // error: post-condition does not hold
    }

    // =========================================================================
    // 13. Positional struct with reads_of<self.0>
    // =========================================================================

    struct Action(|address|u64 has copy+store+drop) has key, drop;
    spec Action {
        reads_of<self.0> Counter;
        // Use self.0 in invariant via the field variable directly
        invariant forall a: address: !aborts_of<self.0>(a);
    }

    /// Pack with safe_read_counter: should verify
    fun create_action(): Action {
        Action(safe_read_counter)
    }

    /// Call through positional field — Counter preserved by reads_of
    fun use_action(action_addr: address, target: address): u64 {
        let a = &Action[action_addr];
        (a.0)(target)
    }
    spec use_action {
        requires exists<Action>(action_addr);
        aborts_if !exists<Action>(action_addr);
        ensures Counter[target] == old(Counter[target]);
    }
}
