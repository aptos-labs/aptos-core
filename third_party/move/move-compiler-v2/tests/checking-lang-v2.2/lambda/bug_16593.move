script {
    fun main() {
        let f = |x| |y| x + y;
        assert!(f(1)(2) == 3);
    }
}
