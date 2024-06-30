module 0x876543::M {
    struct R { f: u64, g: u64 }

    fun main() {
        let g: u64;
        let r = R { f: 3, g };
        let R { f: x, g: y } = r;
        let z = y;
        let q = x;
    }

    fun main2() {
        let x0: u64 = 0;
        let y0: u64;
        let r = R { f: x0, g: y0 };
        let R { f: x, g: y } = r;
        let z = y;
        let q = x;
    }

    fun main3() {
        let r: R;
        let R { f: x, g: y } = r;
        let z = y;
        let q = x;
    }

    fun main4() {
        let R { f: x, g: y };
        let z = y;
        let q = x;
    }
}
