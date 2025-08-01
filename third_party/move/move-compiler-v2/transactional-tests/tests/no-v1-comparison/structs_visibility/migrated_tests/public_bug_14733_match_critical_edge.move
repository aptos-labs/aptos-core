// This source tests various scenarios of matches which introduce critical edges, which
// are eliminated by the split-critical-edge-processor. This kind of matches
// are also the only kind of code which can introduce those edges, so we are also
// testing functionality of the processor here.

//# publish
module 0x815::m {
    public enum Entity has drop {
        Person { id: u64 },
        Institution { id: u64, admin: Admin }
    }

    public enum Admin has drop {
        Superuser,
        User(u64)
    }

}

//# publish
module 0x815::m_bug_14733_match_critical_edge {
    use 0x815::m::Entity;
    use 0x815::m::Admin;


    // Ensure function is inlined so we create nested matches in the next one.
    inline fun id(s: &Entity): u64 {
        match (s) {
            Person{id} if *id > 10 => *id,
            Institution{id, ..} => *id,
            _ => 0
        }
    }

    fun admin_id(e: &Entity): u64 {
        match (e) {
            Institution{admin: Admin::Superuser, ..} => 1 + id(e),
            Institution{admin: Admin::User(id), ..} if *id > 10 => *id + id(e),
            Institution{admin: Admin::User(id), ..} if  *id <= 10 => id(e) + 5,
            _ => id(e)

        }
    }

    fun test1(): u64 {
        let e = Entity::Person{id: 22};
        id(&e)
    }

    fun test2(): u64 {
        let e = Entity::Person{id: 10};
        id(&e)
    }

    fun test3(): u64 {
        let e = Entity::Institution { id: 10, admin: Admin::Superuser };
        id(&e)
    }

    fun test4(): u64 {
        let e = Entity::Institution{id: 20, admin: Admin::Superuser};
        admin_id(&e)
    }

    fun test5(): u64 {
        let e = Entity::Institution{id: 20, admin: Admin::User(23)};
        admin_id(&e)
    }

    fun test6(): u64 {
        let e = Entity::Institution{id: 20, admin: Admin::User(8)};
        admin_id(&e)
    }

    fun test7(): u64 {
        let e = Entity::Person{id: 15};
        admin_id(&e)
    }
}

//# run 0x815::m_bug_14733_match_critical_edge::test1

//# run 0x815::m_bug_14733_match_critical_edge::test2

//# run 0x815::m_bug_14733_match_critical_edge::test3

//# run 0x815::m_bug_14733_match_critical_edge::test4

//# run 0x815::m_bug_14733_match_critical_edge::test5

//# run 0x815::m_bug_14733_match_critical_edge::test6

//# run 0x815::m_bug_14733_match_critical_edge::test7
