module 0x99::basic_struct {
  struct Wrapper<T> has key {
    fv: T
  }

  #[persistent]
   fun test(f: &||u64): u64 {
    1
   }

  // not ok: storage mandates struct type
  fun add_resource_with_fv(acc: &signer, f: | &||u64 |u64 has store+drop+key) {
    move_to<| &||u64 |u64 has store+drop+key>(acc, f);
  }

  public fun test_driver(acc: &signer){
    // not ok case: cannot put function values in storage directly
    let f: | &||u64 |u64 has store+drop+key = test;
    add_resource_with_fv(acc, f);
  }
}
