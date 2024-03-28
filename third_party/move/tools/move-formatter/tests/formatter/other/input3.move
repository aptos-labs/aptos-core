module Test {
struct SomeOtherStruct1 has drop {
    some_other_field1: u64,
  }

  struct SomeOtherStruct2 
  has drop {
    some_other_field2: SomeOtherStruct1,
  }

  struct SomeOtherStruct3 has drop {
    some_other_field3: SomeOtherStruct2,
  }
struct SomeOtherStruct4 has drop {
some_other_field4: SomeOtherStruct3,
}

                    struct SomeOtherStruct5 has drop {
                        some_other_field5: SomeOtherStruct4,
                    }

    struct SomeOtherStruct6 has drop {
        some_other_field6: SomeOtherStruct5,
    }
  struct SomeStruct has key, drop, store {
    some_field: SomeOtherStruct6,
  }

  fun acq(addr: address): u64 acquires SomeStruct {
    let val = borrow_global<SomeStruct>(addr);

    val.some_field.  some_other_field6. some_other_field5.     some_other_field4.     some_other_field3.   some_other_field2.   some_other_field1
  }
}
