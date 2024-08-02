//# publish
module 0x42::X {
    public fun tester(a: bool, b: bool): u64 {
        let x = 1;
        { x = x * 2; a } && { x = x * 3; true };
        { x = x * 5; b } || { x = x * 7; false };
        x
    }
}

//# run
script {
use 0x42::X;
fun main() {
    assert!(X::tester(false, false) == 70, 1);
    assert!(X::tester(false, true) == 10, 2);
    assert!(X::tester(true, false) == 210, 3);
    assert!(X::tester(true, true) == 30, 4);
}
}
