//# publish
module 0x42::m {

  enum Data has drop {
    V1{x: u64},
    V2{x: u64, y: bool}
  }

  fun get_y(self: &Data): bool {
    match (self) {
      V2{x: _, y} => *y,
      _ => abort 33
    }
  }

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
    d.get_y()
  }

  fun test_get_y_v2(): bool {
    let d = Data::V2{x: 43, y: true};
    d.get_y()
  }
}

//# run 0x42::m::test_get_x_v1

//# run 0x42::m::test_get_x_v2

//# run 0x42::m::test_get_y_v1

//# run 0x42::m::test_get_y_v2
