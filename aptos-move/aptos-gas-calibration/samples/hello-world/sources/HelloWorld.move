module 0xcafe::test_hello_world {
    use aptos_std::string_utils;

    public entry fun ccalibrate_add_two_num() {
        let num: u64 = 10;
        string_utils::to_string<u64>(&num);
    }

    public entry fun ccalibrate_add_two_num_2() {
        let num: u64 = 20;
        string_utils::to_string<u64>(&num);
    }
}
