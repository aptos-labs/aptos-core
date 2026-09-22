module 0x42::double {
    fun double_n_times(r: &mut u64, n: u64) {
        let i = 0;
        while (i < n) {
            *r = *r * 2;
            i = i + 1;
        }
    }
}
