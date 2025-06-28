script {
    fun main() {
        let _f: || has drop = main;
        let _g: || has drop = || main();
        (main)();
        main();
    }
}
