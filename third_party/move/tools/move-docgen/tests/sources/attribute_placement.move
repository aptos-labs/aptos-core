#[attr1]
address 0x42 {
#[attr2]
#[attr7]
module M {
    #[attr3]
    use 0x42::N;

    #[attr4]
    struct S {}

    #[attr4b]
    #[resource_group(scope = global)]
    struct T {}

    #[attr2]
    #[attr5]
    const C: u64 = 0;

    #[attr6]
    #[resource_group_member(group = std::string::String)]
    public fun foo() { N::bar() }

    #[attr7]
    spec foo {}
}
}

#[attr8]
module 0x42::N {
    #[attr9]
    friend 0x42::M;

    #[attr10]
    public fun bar() {}
}

#[attr11]
script {
    #[attr12]
    use 0x42::M;

    #[attr13]
    const C: u64 = 0;

    #[attr14]
    fun main() {
        M::foo();
    }

    #[attr15]
    spec main { }
}
