script {
fun main(p: bool) {
    let x = 5;
    let ref;
    if (p) {
        ref = &x;
    };
    assert!(*move ref == 5, 42);
}
}

// check: MOVELOC_UNAVAILABLE_ERROR
