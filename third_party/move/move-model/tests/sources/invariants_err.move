module 0x42::M {

  struct S has key {
    x: u64,
  }

  spec S {
    // Expression not a bool
    invariant x + 1;
    // Old expression in data invariant
    invariant old(x) > 0;
  }

  spec module {
    fun rec_fun(c: bool): bool {
        if (c) {
          rec_fun2(c)
        } else {
          spec_var > 0
        }
      }
      fun rec_fun2(c: bool): bool {
         rec_fun(!c)
      }
    }

    // Type instantiation for global memory operations is not a struct type
    invariant exists<u64>(@0x0);
    invariant<T> global<T>(@0x1) == global<T>(@0x2);
}
