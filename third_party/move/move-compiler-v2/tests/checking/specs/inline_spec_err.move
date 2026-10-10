module 0x42::M {

  struct R has key { v: u64 }

  fun invalid_old_exp(x: &mut u64, y: u64) {
    let a = x;
    let b = &mut y;
    *a = *b;
    *b = 0;
    spec {
      assert old(a) == y; // error: local under old
      assert old(b) == 0; // error: local under old
      assert old(x != y);
    }
  }

  fun valid_global_old(a: address, y: u64) {
    let r = &mut R[a];
    r.v = r.v + y;
    spec {
      assert R[a].v == old(R[a].v) + y;
      assert exists<R>(a) == old(exists<R>(a));
      assert old(R[a].v + y) == R[a].v;
      assert forall k in 0..1: old(R[a].v + k) >= k;
    }
  }
}
