script {
    use 0x2::Fail;
    fun fail() {
        Fail::f();
    }
}
