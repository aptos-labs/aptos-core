module 0x42::m {

    fun len(): bool {
        true
    }

    fun f(gallery: &vector<u64>) {
        let len = 5;
        spec {
            assert len(gallery) >= 0; // err is raised here because the built-in one is shadowed.
            assert len();
        };
    }
}
