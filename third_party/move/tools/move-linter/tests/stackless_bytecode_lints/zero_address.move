module 0x42::zero_address_checks {
    fun transfer(_recipient: address) {}

    // Should warn: no guard
    public fun unguarded(recipient: address) {
        transfer(recipient);
    }

    // Should not warn: guard using inequality
    public fun guarded_with_neq(recipient: address) {
        if (recipient != @0x0) {
            transfer(recipient);
        };
    }

    // Should not warn: guard using equality with early abort
    public fun guarded_with_abort(recipient: address) {
        if (recipient == @0x0) {
            abort 1;
        };
        transfer(recipient);
    }

    // Should not warn: guard via assert!
    public fun guarded_with_assert(recipient: address) {
        assert!(recipient != @0x0, 1);
        transfer(recipient);
    }

    // Should warn on final use: branch compares with different literal
    public fun guarded_with_non_zero_literal(recipient: address) {
        if (recipient == @0x1) {
            transfer(recipient);
        };
        transfer(recipient);
    }

    // Should warn when alias used without guard
    public fun alias_without_guard(recipient: address) {
        let recipient_copy = recipient;
        transfer(recipient_copy);
    }

    // Should not warn: alias guarded via negated equality
    public fun alias_guarded(recipient: address) {
        let recipient_copy = recipient;
        if (!(recipient_copy == @0x0)) {
            transfer(recipient_copy);
        };
    }

    fun zero_addr(): address {
        @0x0
    }

    // Should warn: guard is invalidated after reassignment
    public fun guard_then_overwrite(recipient: address) {
        if (recipient != @0x0) {
            recipient = zero_addr();
        };
        transfer(recipient);
    }

    // Should warn: alias guard is invalidated after alias reassignment
    public fun alias_guard_then_overwrite(recipient: address) {
        let recipient_copy = recipient;
        if (recipient != @0x0) {
            recipient_copy = zero_addr();
        };
        transfer(recipient_copy);
    }
}
