module 0x42::known_non_aborting_calls {
    use std::string;

    fun constant(): u64 {
        7
    }
    spec constant(): u64 {
        pragma opaque = true;
        ensures [inferred] result == 7;
        aborts_if [inferred] false;
    }


    fun calls_inferred_no_abort(): u64 {
        constant()
    }
    spec calls_inferred_no_abort(): u64 {
        pragma opaque = true;
        ensures [inferred] result == constant();
        aborts_if [inferred] false;
    }


    fun constructs_empty_string(): bool {
        string::utf8(b"").length() == 0
    }
    spec constructs_empty_string(): bool {
        use 0x1::string;
        pragma opaque = true;
        ensures [inferred] result == (string::length(string::utf8(vector<u8>[])) == 0);
        aborts_if [inferred] aborts_of<string::utf8>(vector<u8>[]);
        aborts_if [inferred] aborts_of<string::length>(string::utf8(vector<u8>[]));
    }

}
/*
Verification: Succeeded.
*/
