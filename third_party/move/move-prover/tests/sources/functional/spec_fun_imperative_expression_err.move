module 0x42::M {
    fun sequential(): u64 {
        let _x = 2;
        let _y = 3;
        while(_y > 0) {
           break
        };
        if (_x > 0) {
            abort(0)
        };
        let _z = if (_x > 5) {
            _x
        } else {
            _y
        };
        _x
    }

    fun m() {
        let _z = 2;
        spec {
            assert _z == sequential();
        };
    }

}
