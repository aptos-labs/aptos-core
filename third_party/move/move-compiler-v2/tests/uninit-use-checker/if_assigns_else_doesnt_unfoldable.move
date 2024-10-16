script {
fun main(p: bool) {
    let x;
    let y;
    if (p) {
        x = 42;
    } else {
        y = 0;
        y;
    };
    assert!(x == 42, 42);
}
}

// check: COPYLOC_UNAVAILABLE_ERROR
