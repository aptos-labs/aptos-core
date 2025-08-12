module 0x99::basic_struct {
  struct Wrapper<T> has key {
    fv: T
  }

  #[persistent]
   fun test(f: &||u64): u64 {
    if (f == f)
        1
    else
        2
   }

  // all abilities satisfied
  fun add_resource_with_struct(acc: &signer, f: | &||u64 |u64 has copy+store+drop) {
    move_to<Wrapper<| &||u64 |u64 has copy+store+drop>>(acc, Wrapper { fv: f});
  }

  public fun test_driver(acc: &signer){
    // ok case
    let f: | &||u64 |u64 has copy+store+drop = test;
    add_resource_with_struct(acc, f);
  }
}
