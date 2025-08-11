script {
  fun example(x: u64): u64 {
    'label1🔥: while (x > 10) {
      loop {
        if (x % 2 == 0) {
          x -= 1;
          continue 'label1;
        } else if (x < 10) {
          break 'label1
        } else
          x -= 2
      }
    };
    x
  }
}
