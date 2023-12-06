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
}
