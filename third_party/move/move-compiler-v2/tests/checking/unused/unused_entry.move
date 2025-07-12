module 0xc0ffee::m {
    entry fun never_called_ok() {
        // never called
    }

    fun never_call_warn() {
        // never called
    }
}

script {
    fun never_called_ok() {
        // never called
    }
}