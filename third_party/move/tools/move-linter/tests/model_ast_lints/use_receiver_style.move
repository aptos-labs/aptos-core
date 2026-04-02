// Tests for the use_receiver_style lint.
// Detects function calls that can use receiver-style (method) syntax.

module 0xc0ffee::provider {
    struct MyStruct has drop {
        value: u64,
    }

    struct Container<T> has drop {
        item: T,
    }

    public fun new(value: u64): MyStruct {
        MyStruct { value }
    }

    public fun new_container<T>(item: T): Container<T> {
        Container { item }
    }

    // Receiver functions (first param is `self`)

    public fun get_value(self: &MyStruct): u64 {
        self.value
    }

    public fun set_value(self: &mut MyStruct, value: u64) {
        self.value = value;
    }

    public fun consume(self: MyStruct): u64 {
        self.value
    }

    public fun is_empty(self: &MyStruct): bool {
        self.value == 0
    }

    public fun get_item<T>(self: &Container<T>): &T {
        &self.item
    }

    // Non-receiver function (first param is NOT `self`)

    public fun create_from(other: &MyStruct): MyStruct {
        MyStruct { value: other.value }
    }

    public fun no_params(): u64 {
        42
    }

    // === Same-module tests ===

    // Should warn: verbose call to immutable receiver function
    public fun test_same_module_immutable_warn(s: MyStruct): u64 {
        get_value(&s)
    }

    // Should warn: verbose call to mutable receiver function
    public fun test_same_module_mutable_warn(s: &mut MyStruct) {
        set_value(s, 42);
    }

    // Should warn: verbose call to by-value receiver function
    public fun test_same_module_consume_warn(s: MyStruct): u64 {
        consume(s)
    }

    // Should warn: verbose call with multiple arguments
    public fun test_same_module_multi_arg_warn(s: &mut MyStruct) {
        set_value(s, 100);
    }

    // Should warn: verbose call to receiver in condition
    public fun test_same_module_in_condition_warn(s: MyStruct): u64 {
        if (is_empty(&s)) { 0 } else { get_value(&s) }
    }

    // Should warn: verbose call to generic receiver function
    public fun test_generic_receiver_warn(c: &Container<u64>): &u64 {
        get_item(c)
    }

    // Should NOT warn: already receiver style
    public fun test_same_module_receiver_no_warn(s: MyStruct): u64 {
        s.get_value()
    }

    // Should NOT warn: already receiver style (mutable)
    public fun test_same_module_receiver_mut_no_warn(s: &mut MyStruct) {
        s.set_value(42);
    }

    // Should NOT warn: already receiver style (consume)
    public fun test_same_module_receiver_consume_no_warn(s: MyStruct): u64 {
        s.consume()
    }

    // Should NOT warn: non-receiver function
    public fun test_non_receiver_no_warn(s: &MyStruct): MyStruct {
        create_from(s)
    }

    // Should NOT warn: function with no parameters
    public fun test_no_params_no_warn(): u64 {
        no_params()
    }

    // Should NOT warn: lint skip attribute
    #[lint::skip(use_receiver_style)]
    public fun test_skip_no_warn(s: MyStruct): u64 {
        get_value(&s)
    }

    // Should NOT warn: already receiver style on generic
    public fun test_generic_receiver_no_warn(c: &Container<u64>): &u64 {
        c.get_item()
    }
}

module 0xc0ffee::consumer {
    use 0xc0ffee::provider;

    // === Cross-module tests ===

    // Should warn: cross-module verbose call to immutable receiver
    public fun test_cross_module_immutable_warn(): u64 {
        let s = provider::new(10);
        provider::get_value(&s)
    }

    // Should warn: cross-module verbose call to mutable receiver
    public fun test_cross_module_mutable_warn() {
        let s = provider::new(10);
        provider::set_value(&mut s, 42);
    }

    // Should warn: cross-module verbose call to by-value receiver
    public fun test_cross_module_consume_warn(): u64 {
        let s = provider::new(10);
        provider::consume(s)
    }

    // Should warn: cross-module verbose call to receiver in condition
    public fun test_cross_module_condition_warn(): u64 {
        let s = provider::new(10);
        if (provider::is_empty(&s)) { 0 } else { provider::get_value(&s) }
    }

    // Should warn: cross-module verbose call to generic receiver
    public fun test_cross_module_generic_warn(c: &provider::Container<u64>): &u64 {
        provider::get_item(c)
    }

    // Should NOT warn: cross-module already receiver style
    public fun test_cross_module_receiver_no_warn(): u64 {
        let s = provider::new(10);
        s.get_value()
    }

    // Should NOT warn: cross-module already receiver style (mutable)
    public fun test_cross_module_receiver_mut_no_warn() {
        let s = provider::new(10);
        s.set_value(42);
    }

    // Should NOT warn: cross-module non-receiver function
    public fun test_cross_module_non_receiver_no_warn(): provider::MyStruct {
        let s = provider::new(10);
        provider::create_from(&s)
    }

    // Should NOT warn: cross-module function with no params
    public fun test_cross_module_no_params_no_warn(): u64 {
        provider::no_params()
    }

    // Should NOT warn: cross-module lint skip
    #[lint::skip(use_receiver_style)]
    public fun test_cross_module_skip_no_warn(): u64 {
        let s = provider::new(10);
        provider::get_value(&s)
    }
}
