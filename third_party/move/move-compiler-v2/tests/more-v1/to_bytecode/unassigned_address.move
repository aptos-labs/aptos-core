// Tests compilation is stopped for unassigned addresses
// Named addresses don't exist at the bytecode level

address UNASSIGNED {
module Ex {}
}

module UNASSIGNED::M {
    struct S {}
    friend UNASSIGNED::N;
    public(friend) fun foo(_: address): S { S{} }
}

module UNASSIGNED::N {
    fun bar(): UNASSIGNED::M::S {
        UNASSIGNED::M::foo(@UNASSIGNED)
    }
}
