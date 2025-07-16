/// Test cases for function values wrapped in structs as keys
//# publish
module 0x99::test_struct {
  use std::signer;

  struct Wrapper<T> has key, drop {
    fv: |T|u64 has copy+store+drop
  }

  #[persistent]
  fun test(f: ||u64 has store+drop): u64 {
    if (f() == 1)
      1
    else
      2
  }

  #[persistent]
  fun test1(): u64 {
    1
  }

  #[persistent]
  fun test2(): u64 {
    2
  }

  // store a function value of type `|T|u64` with `T = ||u64 has store+drop`
  public fun init(acc: &signer){
    let f: | ||u64 has store+drop |u64 has copy+store+drop = |x| test(x);
    move_to(acc, Wrapper { fv: f});
  }

  // check existence of a function value of type `|T|u64` with `T = ||u64 has store+drop`
  // should return true
  public fun test_exist(acc: &signer){
    let exist_res = exists<Wrapper<||u64 has store+drop>>(signer::address_of(acc));
    assert!(exist_res);
  }

  // check existence of a function value of type `|T|u64` with `T = u64`
  // should return false due to incompatible T type
  public fun test_not_exist_1(acc: &signer){
    let exist_res = exists<Wrapper<u64>>(signer::address_of(acc));
    assert!(!exist_res);
  }

  // check existence of a function value of type `|T|u64` with `T = ||u64 has store`
  // should return false due to T missing `drop`
  public fun test_not_exist_2(acc: &signer){
    let exist_res = exists<Wrapper<||u64 has store>>(signer::address_of(acc));
    assert!(!exist_res);
  }

  // check existence of a function value of type `|T|u64` with `T = ||u64 has store`
  // should return false due to T additionally having `copy`
  public fun test_not_exist_3(acc: &signer){
    let exist_res = exists<Wrapper<||u64 has store+drop+copy>>(signer::address_of(acc));
    assert!(!exist_res);
  }

  // borrow function value of type `|T|u64` with `T = ||u64 has store+copy+drop`
  // should fail due to T additionally having `copy`
  public fun test_bad_borrow_from(acc: &signer){
    let f = borrow_global<Wrapper<||u64 has store+copy+drop>>(signer::address_of(acc));
    assert!((f.fv)(test1) == 1);
    assert!((f.fv)(test2) == 2);
  }

  // borrow function value of type `|T|u64` with `T = ||u64 has store+drop`
  // should succeed
  public fun test_borrow_from(acc: &signer){
    let f = borrow_global<Wrapper<||u64 has store+drop>>(signer::address_of(acc));
    assert!((f.fv)(test1) == 1);
    assert!((f.fv)(test2) == 2);
  }

  // move function value of type `|T|u64` with `T = ||u64 has store+copy+drop`
  // should fail due to T additionally having `copy`
  public fun test_bad_move_from(acc: &signer){
    move_from<Wrapper<||u64 has store+copy+drop>>(signer::address_of(acc));
  }

  // move function value of type `|T|u64` with `T = ||u64 has store+drop`
  // should succeed
  public fun test_move_from(acc: &signer){
    move_from<Wrapper<||u64 has store+drop>>(signer::address_of(acc));
  }
}

//# run --verbose --signers 0x1 -- 0x99::test_struct::init

//# run --verbose --signers 0x1 -- 0x99::test_struct::test_exist

//# run --verbose --signers 0x1 -- 0x99::test_struct::test_not_exist_1

//# run --verbose --signers 0x1 -- 0x99::test_struct::test_not_exist_2

//# run --verbose --signers 0x1 -- 0x99::test_struct::test_not_exist_3

//expect to fail
//# run --verbose --signers 0x1 -- 0x99::test_struct::test_bad_borrow_from

//# run --verbose --signers 0x1 -- 0x99::test_struct::test_borrow_from

//expect to fail
//# run --verbose --signers 0x1 -- 0x99::test_struct::test_bad_move_from

//# run --verbose --signers 0x1 -- 0x99::test_struct::test_move_from

// expected to fail as the functon value has been removed above
//# run --verbose --signers 0x1 -- 0x99::test_struct::test_borrow_from


/// Test cases for function values wrapped in enums as keys
//# publish
module 0x99::test_enum {
  use std::signer;

  enum Wrapper<T> has key {
    V1 {fv1: |T|u64 has copy + store},
    V2 {fv1: |T|u64 has copy + drop + store}
  }


  #[persistent]
  fun test(f: ||u64 has store+drop): u64 {
    if (f() == 1)
      1
    else
      2
  }

  #[persistent]
  fun test1(): u64 {
    1
  }

  #[persistent]
  fun test2(): u64 {
    2
  }

  // store a function value of type `|T|u64` with `T = ||u64 has store+drop`
  public fun init(acc: &signer){
    let f1: | ||u64 has store+drop |u64 has copy+store = |x| test(x);
    let v1 = Wrapper::V1{ fv1: f1 };
    move_to(acc, v1);
  }

  // failed store because resource type already exists
  public fun bad_init(acc: &signer){
    let f1: | ||u64 has store+drop |u64 has copy+drop+store = |x| test(x);
    let v2 = Wrapper::V2{ fv1: f1 };
    move_to(acc, v2);
  }

  // check existence of a function value of type `|T|u64` with `T = ||u64 has store+drop`
  // should return true
  public fun test_exist(acc: &signer){
    let exist_res = exists<Wrapper<||u64 has store+drop>>(signer::address_of(acc));
    assert!(exist_res);
  }

  // check existence of a function value of type `|T|u64` with `T = u64`
  // should return true due to incompatible T type
  public fun test_not_exist_1(acc: &signer){
    let exist_res = exists<Wrapper<u64>>(signer::address_of(acc));
    assert!(!exist_res);
  }

  // check existence of a function value of type `|T|u64` with `T = ||u64 has store`
  // should return true due to T missing `drop`
  public fun test_not_exist_2(acc: &signer){
    let exist_res = exists<Wrapper<||u64 has store>>(signer::address_of(acc));
    assert!(!exist_res);
  }

  // check existence of a function value of type `|T|u64` with `T = ||u64 has store`
  // should return true due to T additionally having `copy`
  public fun test_not_exist_3(acc: &signer){
    let exist_res = exists<Wrapper<||u64 has store+drop+copy>>(signer::address_of(acc));
    assert!(!exist_res);
  }

  // borrow function value of type `|T|u64` with `T = ||u64 has store+copy+drop`
  // should fail due to T additionally having `copy`
  public fun test_bad_borrow_from(acc: &signer){
    borrow_global<Wrapper<||u64 has store+copy+drop>>(signer::address_of(acc));
  }

  // borrow function value of type `|T|u64` with `T = ||u64 has store+drop`
  // should succeed
  public fun test_borrow_from(acc: &signer){
    let f = borrow_global<Wrapper<||u64 has store+drop>>(signer::address_of(acc));
    let res = match (f) {
      V1{fv1} => (*fv1)(test1),
      V2{fv1} => (*fv1)(test2)
    };
    assert!(res == 1, 0);
  }

  // move function value of type `|T|u64` with `T = ||u64 has store+copy+drop`
  // should fail due to T additionally having `copy`
  public fun test_bad_move_from(acc: &signer){
    let f = move_from<Wrapper<||u64 has store+copy+drop>>(signer::address_of(acc));
    let res = match (f) {
      V1{fv1} => fv1(test1),
      V2{fv1} => fv1(test2)
    };
    assert!(res == 1, 0);
  }

  // move function value of type `|T|u64` with `T = ||u64 has store+drop`
  // should succeed
  public fun test_move_from(acc: &signer){
    let f = move_from<Wrapper<||u64 has store+drop>>(signer::address_of(acc));
    let res = match (f) {
      V1{fv1} => fv1(test1),
      V2{fv1} => fv1(test2)
    };
    assert!(res == 1, 0);
  }

}

//# run --verbose --signers 0x1 -- 0x99::test_enum::init

//expect to fail
//# run --verbose --signers 0x1 -- 0x99::test_enum::bad_init

//# run --verbose --signers 0x1 -- 0x99::test_enum::test_exist

//# run --verbose --signers 0x1 -- 0x99::test_enum::test_not_exist_1

//# run --verbose --signers 0x1 -- 0x99::test_enum::test_not_exist_2

//# run --verbose --signers 0x1 -- 0x99::test_enum::test_not_exist_3

//expect to fail
//# run --verbose --signers 0x1 -- 0x99::test_enum::test_bad_borrow_from

//# run --verbose --signers 0x1 -- 0x99::test_enum::test_borrow_from

//expect to fail
//# run --verbose --signers 0x1 -- 0x99::test_enum::test_bad_move_from

//# run --verbose --signers 0x1 -- 0x99::test_enum::test_move_from

// expected to fail as the functon value has been removed above
//# run --verbose --signers 0x1 -- 0x99::test_enum::test_move_from
