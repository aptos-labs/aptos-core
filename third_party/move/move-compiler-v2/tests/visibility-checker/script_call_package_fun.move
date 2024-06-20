module 0x42::test {
    public(package) fun foo() {}
}

script {
    fun main() {
        0x42::test::foo()
    }
}
