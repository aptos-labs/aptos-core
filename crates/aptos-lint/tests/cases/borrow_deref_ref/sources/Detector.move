module NamedAddr::counter {
    struct MyResource {
        value: u64,
    }

    public fun test_borrow_deref_ref() {
        let resource = MyResource { value: 10 };

        // Correct usage
        let ref1 = &resource;
        let value1 = *ref1;

        // Unnecessary dereference and then borrow pattern
        let ref2 = &*(&resource);
        let value2 = *ref2;

        let resource_ref = &resource;
        let resource_ref_mut = &mut resource;

        // Unnecessary dereference of a reference pattern
        let value_a = *&resource_ref;
        let value_b = *&mut resource_ref_mut;


        // Other expressions for variety
        let ref3 = &resource;
        let ref4 = &mut resource;
        let value3 = *ref3;
        let value4 = *ref4;

        // Other expressions for variety
        let direct_ref = &resource;
        let direct_ref_mut = &mut resource;
        let direct_value = *direct_ref;
        let direct_value_mut = *direct_ref_mut;

        // More complex expression
        let complex_case = *&*&resource_ref;
    }

}