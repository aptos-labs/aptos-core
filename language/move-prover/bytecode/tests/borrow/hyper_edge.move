// dep: ../../move-stdlib/sources/Vector.move

module 0x2::Collection {
    use Std::Vector;

    struct Collection<T> has drop {
        items: vector<T>,
        owner: address,
    }

    public fun borrow_mut<T>(c: &mut Collection<T>, i: u64): &mut T {
        Vector::borrow_mut(&mut c.items, i)
    }

    public fun make_collection<T>(): Collection<T> {
        Collection {
            items: Vector::empty(),
            owner: @0x2,
        }
    }
}

module 0x2::Test {
    use 0x2::Collection;

    struct Token<phantom T> has drop { value: u64 }

    public fun foo<T>(i: u64) {
        let c = Collection::make_collection<Token<T>>();
        let t = Collection::borrow_mut(&mut c, i);
        t.value = 0;
    }
}
