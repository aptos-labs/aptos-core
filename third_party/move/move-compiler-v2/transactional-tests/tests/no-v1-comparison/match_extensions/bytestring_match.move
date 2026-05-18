//# publish
module 0xc0ffee::m {
    // Test basic byte string matching with b"..." syntax
    public fun test_bstring(x: vector<u8>): u64 {
        match (x) {
            b"hello" => 1,
            b"world" => 2,
            _ => 99,
        }
    }

    // Test hex string matching with x"..." syntax
    public fun test_xstring(x: vector<u8>): u64 {
        match (x) {
            x"deadbeef" => 1,
            x"cafe" => 2,
            _ => 99,
        }
    }

    // Test empty byte string
    public fun test_empty(x: vector<u8>): u64 {
        match (x) {
            b"" => 1,
            _ => 0,
        }
    }

    // Test variable binding (catch-all)
    public fun test_var_binding(x: vector<u8>): vector<u8> {
        match (x) {
            b"special" => b"found",
            other => other,
        }
    }

    // Test guard conditions with byte strings
    public fun test_guard(x: vector<u8>, flag: bool): u64 {
        match (x) {
            b"hello" if flag => 10,
            b"hello" => 20,
            _ => 0,
        }
    }
}

//# run 0xc0ffee::m::test_bstring --args b"hello"

//# run 0xc0ffee::m::test_bstring --args b"world"

//# run 0xc0ffee::m::test_bstring --args b"other"

//# run 0xc0ffee::m::test_xstring --args x"deadbeef"

//# run 0xc0ffee::m::test_xstring --args x"cafe"

//# run 0xc0ffee::m::test_xstring --args x"0000"

//# run 0xc0ffee::m::test_empty --args b""

//# run 0xc0ffee::m::test_empty --args b"notempty"

//# run 0xc0ffee::m::test_var_binding --args b"special"

//# run 0xc0ffee::m::test_var_binding --args b"anything"

//# run 0xc0ffee::m::test_guard --args b"hello" true

//# run 0xc0ffee::m::test_guard --args b"hello" false

//# run 0xc0ffee::m::test_guard --args b"other" true
