module NamedAddr::Detector {
   struct Data has drop {
        value: u64,
    }

    public fun test_function_with_immutable_reference(data: &Data) {
        // Function that only requires an immutable reference
    }

    // public fun test_function_with_mutable_reference(data: &mut Data) {
    //     // Function that requires a mutable reference
    // }

    public fun test_unnecessary_mutable_reference() {
        let data = Data { value: 10 };
        test_function_with_immutable_reference(&mut data) // Should be flagged as unnecessary mutable reference
    }
}
