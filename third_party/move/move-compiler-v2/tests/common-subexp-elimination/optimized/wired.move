//# publish
module 0x42::test {

    struct W has key, store, drop, copy {
        x: u64
    }

    fun greater(self: &W, s: W): bool {
        self.x > s.x
    }

    fun merge(self: &mut W, s: W) {
        self.x += s.x;
    }

    fun foo_2(account: address, w: W) acquires W {
        W[account].merge(w)
    }

    fun test_receiver() acquires W {
        let w = W {
            x: 3
        };
        assert!(!W[@0x1].greater(w), 0);
        foo_2(@0x1, w);
        assert!(W[@0x1].x == 5, 0);
    }
}
