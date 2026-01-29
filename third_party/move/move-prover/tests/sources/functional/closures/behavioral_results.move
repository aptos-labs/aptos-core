// Test cases for result_of behavioral predicate
// result_of<f>(x) returns a deterministic result based on ensures_of<f>(x, y)
// Semantics: result_of<f>(x) == choose y where ensures_of<f>(x, y)
module 0x42::behavioral_results {

    // Test 1: Basic result_of with simple function
    fun apply(f: |u64| u64, x: u64): u64 { f(x) }
    spec apply {
        ensures result == result_of<f>(x);
    }

    // Test 2: result_of with known function
    fun double(x: u64): u64 { x * 2 }
    spec double { ensures result == x * 2; }

    fun test_known(): u64 { double(5) }
    spec test_known {
        ensures result == result_of<double>(5);
    }

    // Test 3: result_of in sequential application
    fun apply_seq(f: |u64| u64 has copy, x: u64): u64 { f(f(x)) }
    spec apply_seq {
        // First application
        let y = result_of<f>(x);
        // Second application uses result of first
        ensures result == result_of<f>(y);
    }

    // Test 4: result_of with multiple parameters
    fun apply2(f: |u64, u64| u64, x: u64, y: u64): u64 { f(x, y) }
    spec apply2 {
        ensures result == result_of<f>(x, y);
    }

    // Test 5: result_of with known function taking multiple parameters
    fun add(x: u64, y: u64): u64 { x + y }
    spec add { ensures result == x + y; }

    fun test_add(): u64 { add(3, 4) }
    spec test_add {
        ensures result == result_of<add>(3, 4);
    }

    // ===== Tests for mutable reference parameters =====

    // Test 6: result_of with void function that has mutable ref param
    // f returns () but modifies x, so result_of returns just the modified value (not a tuple)
    fun apply_void_mut(f: |&mut u64|, x: &mut u64) { f(x) }
    spec apply_void_mut {
        // result_of returns just the modified value (not a tuple since only one output)
        ensures x == result_of<f>(old(x));
    }

    // Test 7: ensures_of with mutable reference parameter
    fun test_ensures_mut(f: |&mut u64| u64, x: &mut u64): u64 { f(x) }
    spec test_ensures_mut {
        // ensures_of takes (input_x, explicit_result, modified_x)
        ensures ensures_of<f>(old(x), result, x);
    }

    // Test 8: result_of with mut ref returning a value - using ensures_of to verify
    // f returns a value AND modifies x, so result_of returns (explicit_result, modified_x) tuple
    fun apply_mut(f: |&mut u64| u64, x: &mut u64): u64 { f(x) }
    spec apply_mut {
        // result_of returns (explicit_result, modified_x) tuple
        // ensures_of takes (input_x, explicit_result, modified_x)
        ensures ensures_of<f>(old(x), result, x);
    }

}
