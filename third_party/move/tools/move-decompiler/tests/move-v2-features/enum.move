module 0x99::enum_simple {
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



module 0x99::enum_complex {
    enum Entity has drop {
        Person { id: u64 },
        Institution { id: u64, admin: Admin }
    }

    enum Admin has drop {
        Superuser,
        User(u64)
    }

    // Ensure function is inlined so we create nested matches in the next one.
    inline fun id(self: &Entity): u64 {
        match (self) {
            Person{id} if *id > 10 => *id,
            Institution{id, ..} => *id,
            _ => 0
        }
    }

    fun admin_id(self: &Entity): u64 {
        match (self) {
            Institution{admin: Admin::Superuser, ..} => 1 + self.id(),
            Institution{admin: Admin::User(id), ..} if *id > 10 => *id + self.id(),
            Institution{admin: Admin::User(id), ..} if  *id <= 10 => self.id() + 5,
            _ => self.id()

        }
    }
}
