//# publish
module 0x42::Test {
    public struct Tester has drop {
        f: u64
    }
}

//# run
script {
use 0x42::Test::Tester;

fun main() {
    let x = Tester { f: 10 };
    assert!(x.f == 10, 70002);

    let i = 0;
    while (i < x.f) {
        x.f = x.f - 1;
        // if inline blocks skips relabelling this will cause a bytecode verifier error
        assert!(9 - x.f == i, 70003);
        i = i + 1
    }
}
}
