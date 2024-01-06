//# run --gas-budget 700
script {
fun main() {
    // nonterminating loop
    for (i in 0..10) {
	i = 0;
	for (j in 0..10) {
            break;
	};
    };
    if (true) () else ();
    assert!(false, 42);
}
}
