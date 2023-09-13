//# print-bytecode
script {
    fun main() {}
}

//# print-bytecode --input=module
module 0x3::N {
    public entry fun ex(_s: signer, _u: u64) {
        abort 0
    }
}
