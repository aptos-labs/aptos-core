spec std::acl {
    spec ACL {
        invariant forall i in 0..len(list), j in 0..len(list): list[i] == list[j] ==> i == j;
    }

    spec fun spec_contains(acl: ACL, addr: address): bool {
        exists a in acl.list: a == addr
    }

    spec contains(acl: &ACL, addr: address): bool {
        ensures result == spec_contains(acl, addr);
    }

    spec add(acl: &mut ACL, addr: address) {
        aborts_if spec_contains(acl, addr) with error::INVALID_ARGUMENT;
        ensures spec_contains(acl, addr);
    }

    spec remove(acl: &mut ACL, addr: address) {
        aborts_if !spec_contains(acl, addr) with error::INVALID_ARGUMENT;
        ensures !spec_contains(acl, addr);
    }

    spec assert_contains(acl: &ACL, addr: address) {
        aborts_if !spec_contains(acl, addr) with error::INVALID_ARGUMENT;
    }
}
