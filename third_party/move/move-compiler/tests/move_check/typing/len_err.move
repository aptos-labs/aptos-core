module 0x42::m {

    fun f_err(gallery: &vector<u64>) {
        let len = 5;
        spec {
            assert len(gallery) >= len;
        };
    }

}
