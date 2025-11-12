module aptos_framework::box_or_inline {
    use aptos_std::table::{Self, Table};
    use std::mem;

    const ONLY_KEY: bool = true;

    enum BoxOrInline<T> has store {
        Inline{ value: T },
        // TODO be able to cache a singleton table handle
        BoxInTable { table: Table<bool, T>},
    }

    public fun new_inline<T: store>(value: T): BoxOrInline<T> {
        BoxOrInline::Inline { value }
    }

    public fun new_box<T: store>(value: T): BoxOrInline<T> {
        let table = table::new();
        table.add(ONLY_KEY, value);
        BoxOrInline::BoxInTable { table }
    }

    public fun borrow<T: store>(self: &BoxOrInline<T>): &T {
        match (self) {
            BoxOrInline::Inline { value } => value,
            BoxOrInline::BoxInTable { table } => table.borrow(ONLY_KEY),
        }
    }

    public fun borrow_mut<T: store>(self: &mut BoxOrInline<T>): &mut T {
        match (self) {
            BoxOrInline::Inline { value } => value,
            BoxOrInline::BoxInTable { table } => table.borrow_mut(ONLY_KEY),
        }
    }

    public fun destroy<T: store>(self: BoxOrInline<T>): T {
        match (self) {
            BoxOrInline::Inline { value } => value,
            BoxOrInline::BoxInTable { table } => {
                let value = table.remove(ONLY_KEY);
                table.destroy_known_empty_unsafe();
                value
            },
        }
    }

    public fun move_to_inline<T: store>(self: &mut BoxOrInline<T>) {
        match (self) {
            BoxOrInline::Inline { value: _ } => {},
            BoxOrInline::BoxInTable { table } => {
                let value = table.remove(ONLY_KEY);
                let BoxOrInline::BoxInTable { table } = mem::replace(self, BoxOrInline::Inline { value });
                table.destroy_known_empty_unsafe();
            },
        }
    }

    public fun move_to_box<T: store>(self: &mut BoxOrInline<T>) {
        match (self) {
            BoxOrInline::Inline { value: _ } => {
                let BoxOrInline::Inline { value } = mem::replace(self, BoxOrInline::BoxInTable { table: table::new() });
                self.table.add(ONLY_KEY, value);
            },
            BoxOrInline::BoxInTable { table: _ } => {},
        }
    }

    #[test]
    fun test_box_or_inline() {
        let value = new_box(Dummy {});
        value.move_to_inline();
        value.move_to_box();
        value.move_to_inline();
        let Dummy {} = value.destroy();
    }

    struct Dummy has store {}
}
