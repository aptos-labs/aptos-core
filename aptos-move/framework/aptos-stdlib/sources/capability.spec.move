spec aptos_std::capability {
    /// Helper specification function to check whether a capability exists at address.
    spec fun spec_has_cap<Feature>(addr: address): bool {
        exists<CapState<Feature>>(addr)
    }

    /// Helper specification function to obtain the delegates of a capability.
    spec fun spec_delegates<Feature>(addr: address): vector<address> {
        global<CapState<Feature>>(addr).delegates
    }
}
