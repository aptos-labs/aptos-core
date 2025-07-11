module 0x99::test_enum {
    // Note: `Shape` has no `drop` ability, so must be destroyed with explicit unpacking.
    enum Shape {
        Circle{radius: u64},
        Rectangle{width: u64, height: u64}
    }

    fun destroy_empty(self: Shape) : bool {
        match (self) {
            Shape::Circle{radius} => true,
            Shape::Rectangle{width, height: _} => false,
        }
    }

    fun example_destroy_shapes() {
        let c = Shape::Circle{radius: 0};
        let r = Shape::Rectangle{width: 0, height: 0};
        c.destroy_empty();
        r.destroy_empty();
    }
}
