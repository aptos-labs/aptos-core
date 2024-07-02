module Storage {
    struct Box<T> {
        value: T
    }
  struct SomeStruct has drop {
    some_field: u64,
  }
    // type u64 is put into angle brackets meaning
    // that we're using Box with type u64
    public fun create_box(value: u64): Box<u64> {
        Box<u64>{ value }
    }    public fun value<T: copy>(box: &Box<T>): T acquires SomeStruct{
        *&box.value
    }
}
