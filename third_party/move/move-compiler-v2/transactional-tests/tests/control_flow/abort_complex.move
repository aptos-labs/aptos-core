//# publish
module 0x42::test {
    fun concat(v0: vector<u8>, v1: vector<u8>): vector<u8> {
        std::vector::append(&mut v0, v1);
        v0
    }

    fun main() {
        abort concat(b"Hello", concat(b", ", concat(b"world", b"!")))
    }
}

//# run --verbose 0x42::test::main
