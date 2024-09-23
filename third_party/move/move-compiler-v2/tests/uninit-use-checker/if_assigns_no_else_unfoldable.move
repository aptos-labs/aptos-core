script {
fun main(p: bool) {
    let x;
    if (p) x = 42;
    assert!(x == 42, 42);
}
}

// check: COPYLOC_UNAVAILABLE_ERROR
