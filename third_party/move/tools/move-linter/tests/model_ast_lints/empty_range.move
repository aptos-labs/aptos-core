module 0xc0ffee::m {

  public fun foo() {

    let v = 0;

    for (i in 10..0) {
      v += i;
    };

    for (i in 0..10) {
      v += i;
    };

    for (i in 10..10) {
      v += i;
    };
  }

  public fun nested_for() {

    let v = 0;

    for (i in 10..0) {
      for (i in 0..10) {
        for (i in 0..10) {
          v += i;
        };
      };
    };

  }

  // This is a desugared for loop - should trigger.
  public fun desugared_for(x: u64) {
    let i = 10;
    let __update_iter_flag: bool = false;
    let __upper_bound_value: u64 = 10;
    loop {
      if (true) {
        if (__update_iter_flag) {
            i = i + 1;
        } else {
            __update_iter_flag = true;
        };
        if (i < __upper_bound_value) {
          // body
        } else {
            break;
        };
      }
    }
  }

  #[lint::skip(empty_range)]
  public fun bar() {
    let v = 0;
    for (i in 10..0) {
      v += i;
    };
  }

}
