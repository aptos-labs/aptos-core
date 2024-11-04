spec aptos_std::capability {
    /// Helper specification function to check whether a capability exists at address.
    spec fun spec_has_cap<Feature>(addr: address): bool {
        exists<CapState<Feature>>(addr)
    }

    /// Helper specification function to obtain the delegates of a capability.
    spec fun spec_delegates<Feature>(addr: address): vector<address> {
        global<CapState<Feature>>(addr).delegates
    }

    /// Helper specification function to check whether a delegated capability exists at address.
    spec fun spec_has_delegate_cap<Feature>(addr: address): bool {
        exists<CapDelegateState<Feature>>(addr)
    }

    spec create<Feature>(owner: &signer, _feature_witness: &Feature) {
        let addr = signer::address_of(owner);
        aborts_if spec_has_cap<Feature>(addr);
        ensures spec_has_cap<Feature>(addr);
    }

    spec acquire<Feature>(requester: &signer, _feature_witness: &Feature): Cap<Feature> {
        let addr = signer::address_of(requester);
        let root_addr = global<CapDelegateState<Feature>>(addr).root;
        include AcquireSchema<Feature>;
        ensures spec_has_delegate_cap<Feature>(addr) ==> result.root == root_addr;
        ensures !spec_has_delegate_cap<Feature>(addr) ==> result.root == addr;
    }

    spec acquire_linear<Feature>(requester: &signer, _feature_witness: &Feature): LinearCap<Feature> {
        let addr = signer::address_of(requester);
        let root_addr = global<CapDelegateState<Feature>>(addr).root;
        include AcquireSchema<Feature>;
        ensures spec_has_delegate_cap<Feature>(addr) ==> result.root == root_addr;
        ensures !spec_has_delegate_cap<Feature>(addr) ==> result.root == addr;
    }

    spec schema AcquireSchema<Feature> {
        addr: address;
        root_addr: address;
        aborts_if spec_has_delegate_cap<Feature>(addr) && !spec_has_cap<Feature>(root_addr);
        aborts_if spec_has_delegate_cap<Feature>(addr) && !vector::spec_contains(spec_delegates<Feature>(root_addr), addr);
        aborts_if !spec_has_delegate_cap<Feature>(addr) && !spec_has_cap<Feature>(addr);
    }

    spec delegate<Feature>(self: Cap<Feature>, _feature_witness: &Feature, to: &signer) {
        let addr = signer::address_of(to);
        ensures spec_has_delegate_cap<Feature>(addr);
        ensures !old(spec_has_delegate_cap<Feature>(addr)) ==> global<CapDelegateState<Feature>>(addr).root == self.root;
        ensures !old(spec_has_delegate_cap<Feature>(addr)) ==> vector::spec_contains(spec_delegates<Feature>(self.root), addr);
    }

    spec revoke<Feature>(self: Cap<Feature>, _feature_witness: &Feature, from: address) {
        ensures !spec_has_delegate_cap<Feature>(from);
        // TODO: this cannot be proved. See issue #7422
        // ensures old(spec_has_delegate_cap<Feature>(from))
        //     ==> !vector::spec_contains(spec_delegates<Feature>(cap.root), from);
    }

    spec remove_element<E: drop>(v: &mut vector<E>, x: &E) {
        // TODO: this cannot be proved. See issue #7422
        // ensures !vector::spec_contains(v, x);
    }

    spec add_element<E: drop>(v: &mut vector<E>, x: E) {
        ensures vector::spec_contains(v, x);
    }
}
