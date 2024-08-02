script {
fun main(p: bool) {
    let x: u64;
    if (p) x = 100;
    assert!(x == 100, 42);
}
}

// check: COPYLOC_UNAVAILABLE_ERROR
