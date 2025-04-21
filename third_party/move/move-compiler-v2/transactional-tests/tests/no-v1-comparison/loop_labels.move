//# run
script {
    fun main() {
        let result = 0;
        'outer: while (result < 100) {
            while (result < 50) {
                'inner: while (result < 30) {
                    result += 1;
                    continue 'outer
               };
               result += 10;
               continue 'outer
            };
            result += 20
        };
        assert!(result == 110);
    }
}
