module 0x42::m {

    struct S has copy, drop { f: vector<u64>, l: u64 }

    fun f(r1: &mut vector<u64>, r2: &u64) {
    }

    //fun g(r1: &u64, r2: &mut vector<u64>) {
    //}

    fun main(s: S) {
        f(&mut s.f, &s.l);
        //g(&s.l, &mut s.f);
    }
}
