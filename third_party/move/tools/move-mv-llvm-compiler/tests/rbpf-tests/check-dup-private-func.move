
// Check that private functions with the same name from two different
// modules (also with the same name) don't have name collision (proper
// visibility).
module 0x100::MX {
    fun my_private() {
    }
    public fun doit1() {
        my_private();
    }
}

module 0x200::MX {
    fun my_private() {
    }
    public fun doit2() {
        my_private();
    }
}

script {
    fun main() {
        0x100::MX::doit1();
        0x200::MX::doit2();
    }
}
