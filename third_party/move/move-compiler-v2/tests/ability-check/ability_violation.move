module 0x42::ability {

    struct Impotent {}

    fun invalid_copy() {
        let x = Impotent {};
        (x, x);
    }
}
