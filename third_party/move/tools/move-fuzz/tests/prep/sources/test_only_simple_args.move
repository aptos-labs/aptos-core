module test::only_simple_args {
    use std::string::String;

    public fun no_args() {}

    public fun simple_types(
        _a: signer,
        _b: bool,
        _c: u8,
        _d: i64,
        _e: address,
        _f: String
    ) {}

    public fun recursive_simple_types(
        _a: vector<u8>,
        _b: vector<vector<address>>,
        _c: vector<vector<vector<i256>>>,
        _d: vector<String>,
        _e: vector<signer>
    ) {}
}
