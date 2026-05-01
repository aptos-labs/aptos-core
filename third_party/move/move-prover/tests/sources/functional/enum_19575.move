module 0x42::example {

      enum Wrapper<T> has copy, drop {
          One { value: T }
      }

      spec Wrapper {
          invariant (self is Wrapper::One) ==> true;
      }
  }
