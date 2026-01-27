//# run --verbose
script {
    fun main() {
        abort {
            let message = b"Hello, world";
            std::vector::push_back<u8>(&mut message, 33);
            message
        }
    }
}
