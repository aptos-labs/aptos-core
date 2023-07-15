module 0xcafe::test_hello_world {
    use aptos_std::string_utils;

    public entry fun calibrate_add_two_num() {
        //let addr: address = @0x1;
        let num: u64 = 10;
        string_utils::to_string<u64>(&num);
        //print<address>(&addr);
    }
}
