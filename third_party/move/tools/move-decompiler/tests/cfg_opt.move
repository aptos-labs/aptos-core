module 0x99::cfg_opt_simple {

    // pattern matched (with both then and else branch)
    fun test1(x: u64, y: u64): u64 {
        loop{
            loop{
                if (x > 1) break;
                if (y > 2) break;
                x = x + 1;
            };
            y = y + 1
        };
        x + y
    }

    // pattern not matched
    fun test2(x: u64, y: u64): u64 {
        loop{
            loop{
                if (x > 1) break;
                x = x + 1;
                if (y > 2) break;
                x = x + 1
            };
            y = y + 1
        };
        x + y
    }

    // pattern matched (without then branch)
    fun test3(x: u64, y: u64): u64 {
        loop{
            loop{
                if (x > 1) break;
                if (y > 2) break;
                x = x + 1;
            };
        };
        x + y
    }

    // pattern matched (without else branch)
    fun test4(x: u64, y: u64): u64 {
        loop{
            loop{
                if (x > 1) break;
                if (y > 2) break;
            };
            y = y + 1
        };
        x + y
    }

    // pattern matched (without then and else branch)
    fun test5(x: u64, y: u64): u64 {
        loop{
            loop{
                if (x > 1) break;
                if (y > 2) break;
            };
        };
        x + y
    }
}


module 0x99::cfg_opt_complex {
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

    fun test1(x: u64, y: u64): u64 {
        loop{
            loop{
                if (x > 1) break;
                if (y > 2) break;
                x = x + 1;
            };
            y = y + 1
        };
        x + y
    }
}
