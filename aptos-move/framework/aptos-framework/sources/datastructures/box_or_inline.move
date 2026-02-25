module aptos_framework::box_or_inline {
    use std::mem;
    use aptos_framework::box::{Self, Box};

    const EBOX_INCORRECTLY_IN_TRANSIENT_STATE: u64 = 1;

    enum BoxOrInline<T> has store {
        Inline{ value: T },
        Boxed { box: Box<T> },
        Transient,
    }

    public fun new_inline<T: store>(value: T): BoxOrInline<T> {
        BoxOrInline::Inline { value }
    }

    public fun new_box<T: store>(value: T): BoxOrInline<T> {
        BoxOrInline::Boxed { box: box::new(value) }
    }

    public fun borrow<T: store>(self: &BoxOrInline<T>): &T {
        match (self) {
            BoxOrInline::Inline { value } => value,
            BoxOrInline::Boxed { box } => box.borrow(),
            BoxOrInline::Transient => abort EBOX_INCORRECTLY_IN_TRANSIENT_STATE,
        }
    }

    public fun borrow_mut<T: store>(self: &mut BoxOrInline<T>): &mut T {
        match (self) {
            BoxOrInline::Inline { value } => value,
            BoxOrInline::Boxed { box } => box.borrow_mut(),
            BoxOrInline::Transient => abort EBOX_INCORRECTLY_IN_TRANSIENT_STATE,
        }
    }

    public fun destroy<T: store>(self: BoxOrInline<T>): T {
        match (self) {
            BoxOrInline::Inline { value } => value,
            BoxOrInline::Boxed { box } => box.destroy(),
            BoxOrInline::Transient => abort EBOX_INCORRECTLY_IN_TRANSIENT_STATE,
        }
    }

    public fun move_to_inline<T: store>(self: &mut BoxOrInline<T>) {
        match (self) {
            BoxOrInline::Inline { value: _ } => {},
            BoxOrInline::Boxed { box: _ } => {
                let BoxOrInline::Boxed { box } = mem::replace(self, BoxOrInline::Transient);
                let BoxOrInline::Transient = mem::replace(self, new_inline(box.destroy()));
            },
            BoxOrInline::Transient => abort EBOX_INCORRECTLY_IN_TRANSIENT_STATE,
        }
    }

    public fun move_to_box<T: store>(self: &mut BoxOrInline<T>) {
        match (self) {
            BoxOrInline::Inline { value: _ } => {
                let BoxOrInline::Inline { value } = mem::replace(self, BoxOrInline::Transient);
                let BoxOrInline::Transient = mem::replace(self, new_box(value));
            },
            BoxOrInline::Boxed { box: _ } => {},
            BoxOrInline::Transient => abort EBOX_INCORRECTLY_IN_TRANSIENT_STATE,
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

    #[test_only]
    struct Dummy has store {}
}
