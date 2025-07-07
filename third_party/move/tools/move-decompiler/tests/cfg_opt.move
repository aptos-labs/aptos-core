module 0x99::cfg_opt_simple {

    // pattern matched 1
    /* Output without optimization
    ```Move
    fun test1(x: u64, y: u64): u64 {
        'l0: loop {
            loop {
                while (!(x > 1 || y > 2)) break 'l0;
                y = y + 1
            };
            break
        };
        x + 1 + y
    }
    ```
    */
    fun test1(x: u64, y: u64): u64 {
       'outer: loop{
            loop{
                if (x > 1) break;
                if (y > 2) break;
                x = x + 1;
                break 'outer
            };
            y = y + 1
        };
        x + y
    }

    // pattern matched 2
    /* Output without optimization
    ```Move
    fun test2(x: u64, y: u64): u64 {
        'l0: loop {
            loop {
                while (!(x > 1 || y > 2)) break 'l0;
                y = y + 1
            };
            break
        };
        x + y
    }
    ```
    */
    fun test2(x: u64, y: u64): u64 {
        'outer: loop{
            loop{
                if (x > 1) break;
                if (y > 2) break;
                break 'outer
            };
            y = y + 1
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
}
