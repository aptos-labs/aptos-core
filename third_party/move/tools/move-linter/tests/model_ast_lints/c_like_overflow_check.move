module 0xc0ffee::m {

  struct Counter has key { i: u64 }


  public fun arg_cmp(a: u64, b: u64) : u64 {
    if (a + b < a) {
      a+  b
    } else {
      abort 1
    }
  }

  public fun fn_cmp(a: u64, b: u64) : u64 {
    if (a_fn(a) + b < a_fn(a)) {
      a+  b
    } else {
      abort 1
    }
  }

  public fun mix_cmp(a: u64, b: u64) : u64 {
    if (a_fn(a) > a_fn(a) + b ) {
      a+  b
    } else {
      abort 1
    }
  }

  public fun get_count(addr: address, b: u64): u64 acquires Counter {
    if (borrow_global<Counter>(addr).i > borrow_global<Counter>(addr).i + b) {
      borrow_global<Counter>(addr).i + b
    } else {
      abort 1
    }
  }


  public fun get_count_2(addr: address, b: u64): u64 acquires Counter {
    let c = borrow_global<Counter>(addr).i;
    if (c > c + b) {
      c + b
    } else {
      abort 1
    }
  }


  public fun a_fn(a: u64) : u64 {
    a
  }
}
