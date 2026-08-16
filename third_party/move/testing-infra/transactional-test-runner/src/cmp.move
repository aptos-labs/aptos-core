module std::cmp {
    enum Ordering has copy, drop {
        Less,
        Equal,
        Greater,
    }

    native public fun compare<T>(first: &T, second: &T): Ordering;

    public fun is_lt(self: &Ordering): bool {
        self is Ordering::Less
    }

    spec compare {
        pragma intrinsic;
    }

    spec Ordering {
        pragma intrinsic;
    }

    spec is_lt {
        pragma intrinsic;
        pragma opaque;
        pragma verify = false;
    }
}
