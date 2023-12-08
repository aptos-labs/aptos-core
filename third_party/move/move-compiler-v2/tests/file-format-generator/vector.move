module 0x42::vector {
    fun create(): vector<u64> {
        vector[1, 2, 3]
    }

    fun test_fold() {
        use std::vector;
        let v = vector[1];
        let accu = vector::fold(v, 0, |_, _| 0 );
        assert!(accu == 0 , 0)
    }

}
