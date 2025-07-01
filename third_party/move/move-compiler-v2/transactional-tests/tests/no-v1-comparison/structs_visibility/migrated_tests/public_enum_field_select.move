//# publish
module 0x42::m {

  public enum Data has drop {
    V1{x: u64},
    V2{x: u64, y: bool}
  }

}

//# publish
module 0x42::test_m {
  use 0x42::m::Data;

  fun test_get_x_v1(): u64 {
    let d = Data::V1{x: 43};
    d.x
  }

  fun test_get_x_v2(): u64 {
    let d = Data::V2{x: 43, y: false};
    d.x
  }

  fun test_get_y_v1(): bool {
    let d = Data::V1{x: 43};
    get_y(&d)
  }

  fun get_y(d: &Data): bool {
    match (d) {
      V2{x: _, y} => *y,
      _ => abort 33
    }
  }

  fun test_get_y_v2(): bool {
    let d = Data::V2{x: 43, y: true};
    get_y(&d)
  }
}

//# run 0x42::test_m::test_get_x_v1

//# run 0x42::test_m::test_get_x_v2

//# run 0x42::test_m::test_get_y_v1

//# run 0x42::test_m::test_get_y_v2
