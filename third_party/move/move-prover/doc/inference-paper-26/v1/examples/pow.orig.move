module 0x42::pow {
  fun pow(base: u64, exp: u64): u64 {
      let result = 1;
      let i = 0;
      while (i < exp) {
          result = result * base;
          i = i + 1;
      };
      result
  }
}
