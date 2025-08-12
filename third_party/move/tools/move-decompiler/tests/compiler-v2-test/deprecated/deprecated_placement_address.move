#[deprecated]
address 0x42 {
module M {
    use 0x41::N;

    struct S {}

    const C: u64 = 0;

    public fun foo(): N::S { let _foo = C + 3; N::bar() }

    spec foo {}
}
}

module 0x41::N {
    struct S has drop { }

    public fun bar(): S { S { } }

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
