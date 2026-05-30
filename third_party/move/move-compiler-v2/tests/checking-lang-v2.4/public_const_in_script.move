// Tests that public/package const is rejected in scripts (parser error).

script {
    public const X: u64 = 42;

    fun main() {}
}
