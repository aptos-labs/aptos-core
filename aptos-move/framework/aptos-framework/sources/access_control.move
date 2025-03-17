/// Allows programmatic access to the Resouce Access Control (RAC) feature of Move.
/// See [AIP-56](https://github.com/aptos-foundation/AIPs/blob/main/aips/aip-56.md).
module std::access_control {

    /// Representation of an access specifier at runtime. This corressponds
    /// to access specifiers annotated at functions in Move, for example
    /// `fun f(addr: address) reads R(*) writes R(addr)`, with the difference
    /// that all symbolic references like `addr` are resolved to actual
    /// runtime values.
    struct AccessSpecifier has store {
        native_data: vector<u8>
    }

    // ---------------------------------------------------------------------------------------------
    // Working with existing acccess specifiers

    /// Returns the access specifier active in the current execution
    /// context. The returned specifier represents the combined restrictions
    /// imposed by all functions on the call stack. This can be used to
    /// freeze and reproduce a particular RAC state.
    public fun capture_acccess_specifier(): AccessSpecifier {
        AccessSpecifier{native_data: current_access_native_data()}
    }

    /// Enter a RAC protected section, making the given specifier active. This
    /// is similar as when a function is called which declares
    /// this access, meaning that this specifier is added to all the
    /// other ones active on the call stack.
    ///
    /// TODO: Once Move 2.2 is in mainnet and can be used in the framework, we can
    /// offer the public function:
    /// ```
    /// public fun `enter(self: &AccessSpecifier, fun: ||) { .. }
    /// ```
    package fun enter_rac_section(self: &AccessSpecifier) {
        native_enter_rac_section(&self.native_data)
    }

    /// Exit a RAC protected section. This will abort if the stack of active
    /// access restrictions is empty, or the top one has not been created
    /// by a call to `enter_rac_section`. The later protectes the integrity
    /// of the semantics of RAC declared in the language, to not be
    /// effected by misplaced calls to this function.
    package fun exit_rac_section(self: &AccessSpecifier) {
        native_enter_rac_section(&self.native_data)
    }

    // ---------------------------------------------------------------------------------------------
    // Building Access Specifiers

    // This is restricted to certain basic patterns at this point. Specifically, it is not
    // possible to explicitly specify resources. This can be extended over time as needed.

    /// Creates an empty access specifier which allows no access, as for a
    /// pure function.
    ///
    /// Use as in
    ///
    /// ```move
    /// let rac = access_control::empty()
    ///     .reads_code_address(true, @myapp)
    ///     .writes_resource_address(false, my_addr)
    /// ```
    public fun empty(): AccessSpecifier {
        AccessSpecifier{native_data: native_empty()}
    }

    /// Adds read access for resources declared at given code address.
    /// Corresponds to `reads <addr>::*` and `!reads <addr>::*`, respectively.
    public fun reads_code_address(self: AccessSpecifier, enable: bool, addr: address): AccessSpecifier {
        AccessSpecifier{ native_data: native_restrict_address(true, true, enable, self.native_data, addr)}
    }

    /// Adds write access for resources declared at given code address.
    /// Corresponds to `writes <addr>::*` and `!writes <addr>::*`, respectively.
    public fun writes_code_address(self: AccessSpecifier, enable: bool, addr: address): AccessSpecifier {
        AccessSpecifier{ native_data: native_restrict_address(true, false, enable, self.native_data, addr)}
    }

    /// Adds read access for resources stored at the given address.
    /// Corresponds to `reads *(<addr>)` and `!reads *(<addr>)`, respectively.
    public fun reads_resource_address(self: AccessSpecifier, enable: bool, addr: address): AccessSpecifier {
        AccessSpecifier{ native_data: native_restrict_address(false, true, enable, self.native_data, addr)}
    }

    /// Adds write access for resources at given code address.
    /// Corresponds to `writes *(<addr>)` and `!writes *(<addr>)`, respectively.
    public fun writes_resource_address(self: AccessSpecifier, enable: bool, addr: address): AccessSpecifier {
        AccessSpecifier{ native_data: native_restrict_address(false, true, enable, self.native_data, addr)}
    }

    // ---------------------------------------------------------------------------------------------
    // Native APIs

    native fun current_access_native_data(): vector<u8>;
    native fun native_empty(): vector<u8>;
    native fun native_enter_rac_section(data: &vector<u8>);
    native fun native_exit_rac_section(data: &vector<u8>);
    native fun native_restrict_address(
        code_else_resource: bool, reads_else_writes: bool, enable: bool, data: vector<u8>, addr: address): vector<u8>;
    native fun native_disable_code_address(data: &mut vector<u8>, addr: address);

}
