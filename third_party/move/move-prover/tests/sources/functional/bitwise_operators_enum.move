// exclude_for: cvc5
address 0x123 {

  module M {

    public fun foo(): u64 {
      let a = A::VA {
        f1: 1
      };
      spec {
          assert (int2bv(a.f1) as u64) == (1 as u64);
      };
      let b = B::VB {
        a: a
      };
      let c = C {
        a: 1,
        b: b.a.f1
      };
      let x = (a.f1 & 0)  | (b.a.f1 & 1) | (c.b ^ 2);
      spec {
        assert x == (((((1 as u64) & (0 as u64)) as u64) | (((1 as u64) & (1 as u64)) as u64) as u64) | (((1 as u64) ^ (2 as u64)) as u64) as u64);
      };
      let _b = B::VB2 {
        a: a,
        a2: 2
      };
      spec {
         assert (((int2bv(_b.a.f1) as u64) & (1 as u64)) as u64) == (1 as u64);
      };
      x
    }

    spec foo {
      pragma bv_ret=b"0";
      ensures result == (((((1 as u64) & (0 as u64)) as u64) | (((1 as u64) & (1 as u64)) as u64) as u64) | (((1 as u64) ^ (2 as u64)) as u64) as u64);
    }

    enum A has copy, drop {
      VA {f1: u64}
    }

    spec A {
      pragma bv=b"1"; // this leads to a warning
    }

    enum B has drop {
      VB {a: A}
      VB2 {a: A, a2: u64}
    }

    struct C has drop {
      a: u64,
      b: u64
    }
    spec C {
      pragma bv=b"1";
    }

  }

}
