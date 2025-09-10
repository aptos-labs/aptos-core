module 0xc0ffee::m {

  struct Counter has key, store { i: u64 }

  struct NestedCounter has key, store {
    n: Counter
  }

  struct HyperNestedCounter has key, store {
    n: NestedCounter
  }

  #[lint::skip(aborting_overflow_checks)]
  public fun ignore(a: u64, b: u64) : u64 {
    if (a + b < a) {
      a + b
    } else {
      abort 1
    }
  }

  public fun arg_cmp(a: u64, b: u64, addr: address) : u64 acquires Counter, NestedCounter, HyperNestedCounter {
    if (a + b < a) {
      a + b
    } else {
      abort 1
    };

    if (a + b > a) {
      a + b
    } else {
      abort 1
    };

    if (a < a + b) {
      a + b
    } else {
      abort 1
    };

    if (a > a + b) {
      a + b
    } else {
      abort 1
    };


    if (a - b > a) {
      a + b
    } else {
      abort 1
    };


    if (a - b < a) { // This should not be warned.
      a + b
    } else {
      abort 1
    };

    if (a > a - b) { // This should not be warned.
      a + b
    } else {
      abort 1
    };


    if (a < a - b) {
      a + b
    } else {
      abort 1
    };

    if (a_fn(a) + b < a_fn(a)) {     // This should not be warned.
      a + b
    } else {
      abort 1
    };

    if (a_fn(a) > a_fn(a) + b ) {    // This should not be warned.
      a + b
    } else {
      abort 1
    };

    let c = borrow_global<Counter>(addr).i;
    if (c > c + b) {
      c + b
    } else {
      abort 1
    };

    let nc = borrow_global<NestedCounter>(addr);
    if (nc.n.i > nc.n.i + b) {
      nc.n.i + b
    } else {
      abort 1
    };

    let hnc = borrow_global<HyperNestedCounter>(addr);
    if (hnc.n.n.i > hnc.n.n.i + b) {
      hnc.n.n.i + b
    } else {
      abort 1
    };

    let i = 0;
    while (a + b > a) {
      i = i + 1;
    };

    if ((a + 1) + (b + 1) < (a + 1)) {
      (a + 1) + (b + 1)
    } else {
      abort 1
    };

    a + b
  }

  public fun overflows(a: u64, b: u64) : bool {
    a + b < a
  }

  public fun a_fn(a: u64) : u64 {
    a
  }
}
