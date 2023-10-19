address 0x42 {
#[deprecated]
module M {
    use 0x41::N;

    #[deprecated]
    struct S {}

    #[deprecated]
    const C: u64 = 0;

    #[deprecated]
    public fun foo(): N::S { let _foo = C + 3; N::bar() }

    spec foo {}
}
}

#[deprecated]
module 0x41::N {
    #[deprecated]
    struct S has drop { }

    #[deprecated]
    public fun bar(): S { S { } }

    #[deprecated]
    public fun consume(_x: S) { }
}

script {
    use 0x42::M;
    use 0x41::N::S;

    fun main() {
	let foo: S = M::foo();
	0x41::N::consume(foo);
    }

    spec main { }
}
