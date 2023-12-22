//# publish
module 0x42::m {
    fun drop_required() {
        let l = 1;
        let r = &mut l;
        let c = true;
        while (c) {
          *r = 2;
          c = false;
        };
        assert!(l == 2, 0);
    }
}

//# run 0x42::m::drop_required
