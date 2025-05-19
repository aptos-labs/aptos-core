script {
    fun main() {
        let f = || {};
        let g = |func| func();
        g(f);
    }
}
