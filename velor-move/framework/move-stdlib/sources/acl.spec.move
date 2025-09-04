spec std::acl {
    spec ACL {
        invariant forall i in 0..len(list), j in 0..len(list): list[i] == list[j] ==> i == j;
    }

    spec fun spec_contains(self: ACL, addr: address): bool {
        exists a in self.list: a == addr
    }

    spec contains(self: &ACL, addr: address): bool {
        ensures result == spec_contains(self, addr);
    }

    spec add(self: &mut ACL, addr: address) {
        aborts_if spec_contains(self, addr) with error::INVALID_ARGUMENT;
        ensures spec_contains(self, addr);
    }

    spec remove(self: &mut ACL, addr: address) {
        aborts_if !spec_contains(self, addr) with error::INVALID_ARGUMENT;
        ensures !spec_contains(self, addr);
    }

    spec assert_contains(self: &ACL, addr: address) {
        aborts_if !spec_contains(self, addr) with error::INVALID_ARGUMENT;
    }
}
