module account::test_module {
    use std::option;

    struct OptionTest has copy, drop {
        o: option::Option<u64>,
    }

    #[view]
    public fun return_none(): option::Option<u64> {
        option::none()
    }

    #[view]
    public fun return_some(): option::Option<u64> {
        option::some(1)
    }

    #[view]
    public fun return_option_test(): OptionTest {
        OptionTest { o: option::some(2) }
    }

    #[view]
    public fun return_vector_of_option(): vector<option::Option<u64>> {
        vector[option::some(3), option::none()]
    }

}
