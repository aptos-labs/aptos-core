script {
fun main(p: bool) {
    let x;
    let y;
    if (p) {
        y = 0;
    } else {
        x = 42;
        x;
    };
    assert!(y == 0, 42);
}
}

// check: COPYLOC_UNAVAILABLE_ERROR
