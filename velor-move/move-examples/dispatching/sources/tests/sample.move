#[test_only]
module dispatching::sample {
    use std::string;

    use velor_framework::function_info;

    use dispatching::engine;
    use dispatching::sample_callback;
    use dispatching::storage;

    #[test(publisher = @dispatching)]
    fun verify_success(publisher: &signer) {
        setup(publisher);
        engine::dispatch<sample_callback::Test>(sample_callback::verify_value());
    }

    #[test(publisher = @dispatching)]
    #[expected_failure(abort_code = 0, location = dispatching::sample_callback)]
    fun verify_abort(publisher: &signer) {
        setup(publisher);
        engine::dispatch<sample_callback::Test>(sample_callback::abort_value());
    }

    fun setup(publisher: &signer) {
        storage::init_module_for_testing(publisher);
        let cb = function_info::new_function_info(
            publisher,
            string::utf8(b"sample_callback"),
            string::utf8(b"callback"),
        );
        storage::register(cb, sample_callback::new_proof());
    }
}
