module 0x1::acl {
    struct ACL has copy, drop, store {
        list: vector<address>,
    }
    
    public fun contains(arg0: &ACL, arg1: address) : bool {
        0x1::vector::contains<address>(&arg0.list, &arg1)
    }
    
    public fun empty() : ACL {
        ACL{list: 0x1::vector::empty<address>()}
    }
    
    public fun remove(arg0: &mut ACL, arg1: address) {
        let (v0, v1) = 0x1::vector::index_of<address>(&mut arg0.list, &arg1);
        assert!(v0, 0x1::error::invalid_argument(1));
        0x1::vector::remove<address>(&mut arg0.list, v1);
    }
    
    public fun add(arg0: &mut ACL, arg1: address) {
        assert!(!0x1::vector::contains<address>(&mut arg0.list, &arg1), 0x1::error::invalid_argument(0));
        0x1::vector::push_back<address>(&mut arg0.list, arg1);
    }
    
    public fun assert_contains(arg0: &ACL, arg1: address) {
        assert!(contains(arg0, arg1), 0x1::error::invalid_argument(1));
    }
    
    // decompiled from Move bytecode v6
}
