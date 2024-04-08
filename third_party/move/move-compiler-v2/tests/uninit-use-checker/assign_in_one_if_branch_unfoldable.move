script {
fun main(p1: bool, p2: bool) {
    let x;
    let y;
    if (p1) x = 5 else ();
    if (p2) y = 5;
    x == y;
}
}

// check: COPYLOC_UNAVAILABLE_ERROR
