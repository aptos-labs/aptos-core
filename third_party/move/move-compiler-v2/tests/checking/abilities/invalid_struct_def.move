module 0x42::ability {
	struct Foo<T: key> { x: T }

	struct Bar { x: Foo<u8> }

	struct Baz<T> { x: Foo<T> }

	struct Impotent {}

	struct Omnipotent has copy, drop, store, key {}

	struct HasKey has key {}

	struct InValidHasKey has key {
		x: HasKey
	}

	struct HasDrop has drop {
		x: Impotent
	}

	struct ConditionalDrop<T> has drop {
		x: T
	}

	struct ConditionalDropInvalid<T> has drop {
		x: ConditionalDrop<T>,
		y: Impotent,
	}

	struct S<T> has drop {
		y: T,
		x: ConditionalDrop<Impotent>
	}

	struct S1<T> {
		x: Foo<T>,
	}

	struct S2<phantom T> {
		x: T
	}

	struct S3<phantom T> {
		x: S<T>
	}

	struct S4<phantom T> has drop {}

	struct S5 has drop {
		x: S4<Impotent>
	}
}
