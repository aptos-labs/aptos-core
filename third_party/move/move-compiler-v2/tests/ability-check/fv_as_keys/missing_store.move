module 0x99::basic_struct {
  struct Wrapper<T> has key {
    fv: T
  }

  #[persistent]
  fun test(f: &||u64): u64 {
    1
  }

  // all abilities satisfied
  fun add_resource_with_struct(acc: &signer, f: | &||u64 |u64 has copy+store+drop) {
    move_to<Wrapper<| &||u64 |u64 has copy+store+drop>>(acc, Wrapper { fv: f});
  }

  public fun test_driver(acc: &signer){
    // not ok case: cannot store functions without `store`
    let f: | &||u64 |u64 has copy+drop = test;
    add_resource_with_struct(acc, f);
  }
}
