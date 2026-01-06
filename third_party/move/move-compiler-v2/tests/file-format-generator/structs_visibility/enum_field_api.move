module 0x42::m {

  friend 0x42::m2;

  friend enum Data has drop {
    V1{x: u64},
    V2{x: u64, y: bool}
    V3
  }

  friend enum Data2<T1, T2> has drop {
    V1{x: T1},
    V2{y: u64, x: T2}
    V3
  }

}

module 0x42::m2 {

  use 0x42::m::Data;
  use 0x42::m::Data2;

  fun test_v1() {
    let d = Data::V1{x: 43};
    assert!(d.x == 43, 1);
    let Data::V1{x} = &d;
    assert!(*x == 43, 2);
    let data_x = &mut d;
    let ref_x = &data_x.x;
    assert!(*ref_x == 43, 3);
  }

  fun test_v1_mut_borrow() {
    let d = Data::V1{x: 43};
    let r = &mut d.x;
    *r = 44;
    assert!(d.x == 44, 1);
    let r2 = &mut d;
    r2.x = 45;
    assert!(d.x == 45, 3);
  }

  fun test_v2_mut_borrow() {
    let d = Data::V2{x: 43, y: true};
    let mut_ref_d = &mut d;
    let ref_x = &mut_ref_d.x;
    assert!(*ref_x == 43, 1);
    let ref_y = &mut_ref_d.y;
    assert!(*ref_y == true, 2);
  }

  fun test_data2_mut_borrow() {
    let d = Data2::V2<u64, u64>{y: 43, x: 44};
    d.x = 45;
  }


}
