module 0x42::TestSchemaName {

    fun with_name_conflict(b: u64) {
        assert!(b > 0, 1);
    }
    spec with_name_conflict {
        let c = b;
        include TestNameConflict {
            x: c + 3
        };
    }

    fun no_conflict(b: u64) {
        assert!(b > 0, 1);
    }
    spec no_conflict {
        let d = b;
        include TestNameConflict {
            x: d + 3
        };
    }

    spec schema TestNameConflict {
        x: u64;
        let c = x > 3;
        aborts_if !(x > 3);
    }

}
