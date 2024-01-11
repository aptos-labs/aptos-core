module 0x42::m {
    friend 0x42::o;

    public inline fun foo(): u64 {
        bar()
    }

    inline fun inaccessible(): u64 {
        bar()
    }

    public(friend) inline fun friend_accessible(): u64 {
        bar()
    }

    public(friend) fun bar(): u64 { 42 }
}

module 0x42::m_nonfriend {
    public inline fun foo(): u64 {
        bar()
    }

    inline fun inaccessible(): u64 {
        bar()
    }

    public(friend) inline fun friend_accessible(): u64 {
        bar()
    }

    public(friend) fun bar(): u64 { 42 }
}

module 0x42::o {
    use 0x42::m;
    use 0x42::m_nonfriend;
    friend 0x42::n;

    public inline fun foo(): u64 {
        m::foo();
        m_nonfriend::foo();
	bar();
	m::friend_accessible();
	m_nonfriend::friend_accessible();
	m::bar();
	m_nonfriend::bar()
    }

    inline fun inaccessible(): u64 {
        m::foo();
        m_nonfriend::foo();
	bar();
	m::friend_accessible();
	m_nonfriend::friend_accessible();
	m::bar();
	m_nonfriend::bar()
    }

    public(friend) inline fun friend_accessible(): u64 {
        m::foo();
        m_nonfriend::foo();
	bar();
	m::friend_accessible();
	m_nonfriend::friend_accessible();
	m::bar();
	m_nonfriend::bar()
    }

    fun bar(): u64 { 42 }
}

module 0x42::o_nonfriend {
    use 0x42::m;
    use 0x42::m_nonfriend;

    public inline fun foo(): u64 {
        m::foo();
        m_nonfriend::foo();
	bar();
	m::friend_accessible();
	m_nonfriend::friend_accessible();
	m::bar();
	m_nonfriend::bar()
    }

    inline fun inaccessible(): u64 {
        m::foo();
        m_nonfriend::foo();
	bar();
	m::friend_accessible();
	m_nonfriend::friend_accessible();
	m::bar();
	m_nonfriend::bar()
    }

    public(friend) inline fun friend_accessible(): u64 {
        m::foo();
        m_nonfriend::foo();
	bar();
	m::friend_accessible();
	m_nonfriend::friend_accessible();
	m::bar();
	m_nonfriend::bar()
    }

    fun bar(): u64 { 42 }
}

module 0x42::n {
    use 0x42::o;
    use 0x42::o_nonfriend;

    public fun test() {
        assert!(o::foo() == 42, 1);
	assert!(o::inaccessible() == 42, 1);
	assert!(o::friend_accessible() == 42, 1);
    }

    public fun test2() {
        assert!(o_nonfriend::foo() == 42, 1);
	assert!(o_nonfriend::inaccessible() == 42, 1);
	assert!(o_nonfriend::friend_accessible() == 42, 1);
    }
}
