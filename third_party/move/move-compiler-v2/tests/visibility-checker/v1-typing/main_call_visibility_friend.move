address 0x2 {
module X {
    public(friend) fun foo() {}

    public fun bar() {}

    fun baz() {}
}
}

script {
fun main() {
    0x2::X::foo();
    0x2::X::bar();
    0x2::X::foo();
    0x2::X::baz();
}
}
