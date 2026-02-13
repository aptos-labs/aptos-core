/// Tests for temporal state labels in spec inference.
/// Verifies that pre-state (@label) vs post-state (no label) is correctly inferred.
///
module 0x42::state_labels {
    struct Resource has key {
        value: u64,
    }

    struct Container has key {
        inner: u64,
    }

    // =========================================================================
    // Basic state label tests
    // =========================================================================

    // MoveFrom: return value must reference pre-state (resource is removed)
    // Expected:
    //   ensures result == global<Resource, @pre>(addr)
    //   aborts_if !exists<Resource, @pre>(addr)
    fun remove_resource(addr: address): Resource acquires Resource {
        move_from<Resource>(addr)
    }
    spec remove_resource(addr: address): Resource {
        ensures [inferred] result == global<Resource>(addr);
        ensures [inferred] !exists<Resource>(addr);
        aborts_if [inferred] !exists<Resource>(addr);
        modifies [inferred] global<Resource>(addr);
    }


    // MoveTo: abort checks pre-state, ensures reference post-state
    // Expected:
    //   ensures exists<Resource>(signer_addr)
    //   ensures global<Resource>(signer_addr) == Resource { value }
    //   aborts_if exists<Resource, @pre>(signer_addr)
    fun publish_resource(account: &signer, value: u64) {
        move_to(account, Resource { value });
    }
    spec publish_resource(account: &signer, value: u64) {
        ensures [inferred] exists<Resource>(0x1::signer::address_of(account));
        ensures [inferred] global<Resource>(0x1::signer::address_of(account)) == Resource{value: value};
        aborts_if [inferred] exists<Resource>(0x1::signer::address_of(account));
        modifies [inferred] global<Resource>(0x1::signer::address_of(account));
    }


    // Resource indexing (immutable): abort checks pre-state
    // Expected:
    //   ensures result == global<Resource>(addr).value
    //   aborts_if !exists<Resource, @pre>(addr)
    fun read_resource(addr: address): u64 acquires Resource {
        Resource[addr].value
    }
    spec read_resource(addr: address): u64 {
        ensures [inferred] result == global<Resource>(addr).value;
        aborts_if [inferred] !exists<Resource>(addr);
    }


    // =========================================================================
    // Behavioral predicates with state-modifying callees
    // =========================================================================

    // Call to state-modifying function (remove_resource)
    // The callee removes state, so result_of should capture pre-state semantics
    // Expected:
    //   ensures result == result_of(remove_resource, addr)
    //   aborts_if aborts_of(remove_resource, addr)
    fun call_remove(addr: address): Resource acquires Resource {
        remove_resource(addr)
    }
    spec call_remove(addr: address): Resource {
        ensures [inferred] result == result_of<remove_resource>(addr);
        aborts_if [inferred] aborts_of<remove_resource>(addr);
    }


    // Call to state-creating function (publish_resource)
    // The callee creates state, behavioral predicates abstract over this
    // Expected:
    //   aborts_if aborts_of(publish_resource, account, value)
    fun call_publish(account: &signer, value: u64) {
        publish_resource(account, value)
    }
    spec call_publish(account: &signer, value: u64) {
        ensures [inferred] ensures_of<publish_resource>(account, value);
        aborts_if [inferred] aborts_of<publish_resource>(account, value);
    }


    // =========================================================================
    // Multiple resources
    // =========================================================================

    // Remove one resource, create another at different type
    // Tests independent state tracking
    // Expected:
    //   ensures result == global<Resource, @pre>(addr)
    //   ensures exists<Container>(account)
    //   aborts_if !exists<Resource, @pre>(addr)
    //   aborts_if exists<Container, @pre>(account)
    fun swap_resources(account: &signer, addr: address): Resource acquires Resource {
        let r = move_from<Resource>(addr);
        move_to(account, Container { inner: r.value });
        r
    }
    spec swap_resources(account: &signer, addr: address): Resource {
        ensures [inferred] result == global<Resource>(addr);
        ensures [inferred] exists<Container>(0x1::signer::address_of(account));
        ensures [inferred] global<Container>(0x1::signer::address_of(account)) == Container{inner: global<Resource>(addr).value};
        ensures [inferred] !exists<Resource>(addr);
        aborts_if [inferred] exists<Container>(0x1::signer::address_of(account));
        aborts_if [inferred] !exists<Resource>(addr);
        modifies [inferred] global<Container>(0x1::signer::address_of(account));
        modifies [inferred] global<Resource>(addr);
    }


    // =========================================================================
    // Conditional state operations
    // =========================================================================

    // Conditional remove: only removes if condition is true
    // Tests path-conditional state labels
    // Expected (with path conditions):
    //   ensures cond ==> result == global<Resource, @pre>(addr)
    //   ensures !cond ==> result == Resource { value: 0 }
    //   aborts_if cond && !exists<Resource, @pre>(addr)
    fun conditional_remove(addr: address, cond: bool): Resource acquires Resource {
        if (cond) {
            move_from<Resource>(addr)
        } else {
            Resource { value: 0 }
        }
    }
    spec conditional_remove(addr: address, cond: bool): Resource {
        ensures [inferred] cond ==> result == global<Resource>(addr);
        ensures [inferred] cond ==> !exists<Resource>(addr);
        ensures [inferred] !cond ==> result == Resource{value: 0};
        aborts_if [inferred] cond && !exists<Resource>(addr);
        modifies [inferred] global<Resource>(addr);
    }


    // Conditional publish: only publishes if resource doesn't exist
    // Tests exists check with state creation
    fun safe_publish(account: &signer, addr: address, value: u64) {
        if (!exists<Resource>(addr)) {
            move_to(account, Resource { value });
        }
    }
    spec safe_publish(account: &signer, addr: address, value: u64) {
        ensures [inferred] !exists<Resource>(addr) ==> exists<Resource>(0x1::signer::address_of(account));
        ensures [inferred] !exists<Resource>(addr) ==> global<Resource>(0x1::signer::address_of(account)) == Resource{value: value};
        aborts_if [inferred] !exists<Resource>(addr) && exists<Resource>(0x1::signer::address_of(account));
        modifies [inferred] global<Resource>(0x1::signer::address_of(account));
    }


    // =========================================================================
    // Chained state operations
    // =========================================================================

    // Remove and republish at same address (via signer)
    // Tests state transition: exists -> !exists -> exists
    // Expected:
    //   ensures exists<Resource>(account)
    //   ensures global<Resource>(account).value == global<Resource, @pre>(addr).value + 1
    //   aborts_if !exists<Resource, @pre>(addr)
    fun increment_resource(account: &signer, addr: address) acquires Resource {
        let r = move_from<Resource>(addr);
        let new_value = r.value + 1;
        let Resource { value: _ } = r;
        move_to(account, Resource { value: new_value });
    }
    spec increment_resource(account: &signer, addr: address) {
        ensures [inferred] exists<Resource>(0x1::signer::address_of(account));
        ensures [inferred] global<Resource>(0x1::signer::address_of(account)) == Resource{value: global<Resource>(addr).value + 1};
        ensures [inferred] !exists<Resource>(addr);
        aborts_if [inferred] exists<Resource>(0x1::signer::address_of(account));
        aborts_if [inferred] global<Resource>(addr).value > MAX_U64 - 1;
        aborts_if [inferred] !exists<Resource>(addr);
        modifies [inferred] global<Resource>(0x1::signer::address_of(account));
        modifies [inferred] global<Resource>(addr);
    }


    // =========================================================================
    // Read after write through reference
    // =========================================================================

    // Borrow, modify, then return modified value
    // Expected:
    //   ensures result == new_value
    //   aborts_if !exists<Resource, @pre>(addr)
    fun update_and_read(addr: address, new_value: u64): u64 acquires Resource {
        let r = &mut Resource[addr];
        r.value = new_value;
        r.value
    }
    spec update_and_read(addr: address, new_value: u64): u64 {
        ensures [inferred] result == global<Resource>(addr).value;
        ensures [inferred] global<Resource>(addr) == update_field(old(global<Resource>(addr)), value, new_value);
        aborts_if [inferred] !exists<Resource>(addr);
        modifies [inferred] global<Resource>(addr);
    }


    // =========================================================================
    // Multiple operations on same resource
    // =========================================================================

    // Read value, then remove resource
    // Expected:
    //   ensures result == global<Resource, @pre>(addr).value
    //   aborts_if !exists<Resource, @pre>(addr)
    fun read_then_remove(addr: address): u64 acquires Resource {
        let val = Resource[addr].value;
        let Resource { value: _ } = move_from<Resource>(addr);
        val
    }
    spec read_then_remove(addr: address): u64 {
        ensures [inferred] result == global<Resource>(addr).value;
        ensures [inferred] !exists<Resource>(addr);
        aborts_if [inferred] !exists<Resource>(addr);
        modifies [inferred] global<Resource>(addr);
    }


    // Check exists, then remove if exists
    // Expected:
    //   ensures result == exists<Resource, @pre>(addr)
    //   (conditional abort based on exists check)
    fun check_and_remove(addr: address): bool acquires Resource {
        let e = exists<Resource>(addr);
        if (e) {
            let Resource { value: _ } = move_from<Resource>(addr);
        };
        e
    }
    spec check_and_remove(addr: address): bool {
        ensures [inferred] result == exists<Resource>(addr);
        ensures [inferred] !exists<Resource>(addr);
    }


    // =========================================================================
    // Behavioral predicates with intermediate states
    // =========================================================================

    // Call after state modification:
    // First creates resource, then calls read_resource
    // The result_of(read_resource, addr) should be evaluated at intermediate state
    // where the resource exists (not the pre-state where it may not exist)
    // Expected:
    //   ensures result == result_of<read_resource, @intermediate>(addr)
    //   aborts_if exists<Resource, @pre>(account)  // from MoveTo
    //   aborts_if aborts_of<read_resource, @intermediate>(addr)  // should NOT abort if we just created it
    fun create_then_read_same(account: &signer, addr: address): u64 acquires Resource {
        // Creates resource at account's address
        move_to(account, Resource { value: 42 });
        // Then reads from addr (which may be same or different)
        read_resource(addr)
    }
    spec create_then_read_same(account: &signer, addr: address): u64 {
        ensures [inferred] result == result_of<read_resource>(addr);
        ensures [inferred] exists<Resource>(0x1::signer::address_of(account));
        ensures [inferred] global<Resource>(0x1::signer::address_of(account)) == Resource{value: 42};
        aborts_if [inferred] aborts_of<read_resource>(addr);
        aborts_if [inferred] exists<Resource>(0x1::signer::address_of(account));
        modifies [inferred] global<Resource>(0x1::signer::address_of(account));
    }


    // Chained function calls with state dependency:
    // The second call sees the state after the first call modified it
    // Expected:
    //   ensures result == result_of<read_resource, @s2>(addr)
    //   where @s2 is state after remove_resource
    fun remove_then_try_read(addr1: address, addr2: address): u64 acquires Resource {
        // Remove resource from addr1
        let Resource { value: _ } = remove_resource(addr1);
        // Try to read from addr2 (intermediate state: addr1 no longer has resource)
        read_resource(addr2)
    }
    spec remove_then_try_read(addr1: address, addr2: address): u64 {
        ensures [inferred] result == result_of<read_resource>(addr2);
        aborts_if [inferred] aborts_of<read_resource>(addr2);
        aborts_if [inferred] aborts_of<remove_resource>(addr1);
    }


    // Multiple function calls - each sees different intermediate state
    // Expected:
    //   result_of@@s1 for first call
    //   result_of@@s2 for second call (where s2 is after first call's effects)
    fun double_remove(addr1: address, addr2: address): (Resource, Resource) acquires Resource {
        let r1 = remove_resource(addr1);  // evaluated at @pre
        let r2 = remove_resource(addr2);  // evaluated at state after first remove
        (r1, r2)
    }
    spec double_remove(addr1: address, addr2: address): (Resource, Resource) {
        ensures [inferred] result_1 == result_of<remove_resource>(addr1)@at_6;
        ensures [inferred] result_2 == at_6@result_of<remove_resource>(addr2);
        aborts_if [inferred] at_6@aborts_of<remove_resource>(addr2);
        aborts_if [inferred] aborts_of<remove_resource>(addr1);
    }


    // Behavioral predicate referencing modified state through sequence:
    // publish modifies state, then call_publish would see that modified state
    fun nested_publish(account1: &signer, account2: &signer, v1: u64, v2: u64) {
        // First publish - evaluated at @pre
        publish_resource(account1, v1);
        // Second publish - should be evaluated at intermediate state after first
        publish_resource(account2, v2);
    }
    spec nested_publish(account1: &signer, account2: &signer, v1: u64, v2: u64) {
        ensures [inferred] at_10@ensures_of<publish_resource>(account2, v2);
        ensures [inferred] ensures_of<publish_resource>(account1, v1)@at_10;
        aborts_if [inferred] at_10@aborts_of<publish_resource>(account2, v2);
        aborts_if [inferred] aborts_of<publish_resource>(account1, v1);
    }

}
// TODO(#18762): state labels do not yet work in verification pipeline and produce expected
// boogie errors.
/*
Verification: [internal] boogie exited with compilation errors:
state_labels.enriched.bpl(6318,22): Error: cannot refer to a global variable in this context: $42_state_labels_Resource_$memory
state_labels.enriched.bpl(6322,38): Error: cannot refer to a global variable in this context: $42_state_labels_Resource_$memory
state_labels.enriched.bpl(6343,22): Error: cannot refer to a global variable in this context: $42_state_labels_Resource_$memory
state_labels.enriched.bpl(6347,60): Error: cannot refer to a global variable in this context: $42_state_labels_Resource_$memory
state_labels.enriched.bpl(6347,120): Error: cannot refer to a global variable in this context: $42_state_labels_Resource_$memory
state_labels.enriched.bpl(6368,21): Error: cannot refer to a global variable in this context: $42_state_labels_Resource_$memory
state_labels.enriched.bpl(6372,21): Error: cannot refer to a global variable in this context: $42_state_labels_Resource_$memory
state_labels.enriched.bpl(6372,137): Error: cannot refer to a global variable in this context: $42_state_labels_Resource_$memory
8 name resolution errors detected in state_labels.enriched.bpl
*/
