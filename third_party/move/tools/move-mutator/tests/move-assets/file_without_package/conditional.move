module 0xCAFE::Cond {
    fun main() {
        let a = true;
        if (a) {
            let b = 10;
        };
    }

    fun main2() {
        let a = true;
        if (a) {
            let b = 10;
        } else {
            let c = 20;
        };
    }

    fun main3() {
        let a = true;
        if (a) {
            let b = 10;
        } else if (a) {
            let c = 20;
        } else {
            let d = 30;
        };
    }
}