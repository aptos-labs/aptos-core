#[test_only]
module dispatching::sample_callback {
    use std::option;

    use velor_framework::object::Object;

    use dispatching::storage;

    friend dispatching::sample;

    struct Test has drop {}

    public(friend) fun new_proof(): Test {
      Test {}
    }

    public fun callback<T: key>(_metadata: Object<T>): option::Option<u128> {
        let value = storage::retrieve(new_proof());
        assert!(value == verify_value(), 0);
        option::none()
    }

    public fun verify_value(): vector<u8> {
        b"success"
    }

    public fun abort_value(): vector<u8> {
        b"fail"
    }
}
