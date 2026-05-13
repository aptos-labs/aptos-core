// Suppression of unused_function for package functions
module 0x42::pkg_suppressed {
    #[lint::skip(unused_function)]
    package fun skipped_pkg(): u64 { 1 }

    #[deprecated]
    package fun deprecated_pkg(): u64 { 2 }

    #[view]
    package fun view_pkg(): u64 { 3 }
}

// Suppression of unused_function for friend functions
module 0x42::friend_suppressed {
    #[lint::skip(unused_function, needless_visibility)]
    friend fun skipped_friend(): u64 { 1 }

    #[deprecated]
    friend fun deprecated_friend(): u64 { 2 }

    #[view]
    friend fun view_friend(): u64 { 3 }
}

// Suppression of needless_visibility for same-module-only friend
module 0x42::needless_friend {
    friend 0x42::needless_friend_caller;

    #[lint::skip(needless_visibility)]
    friend fun skipped_needless_friend(): u64 { 1 }

    public fun caller(): u64 {
        skipped_needless_friend()
    }
}

module 0x42::needless_friend_caller {}

// Suppression of needless_visibility for same-module-only package
module 0x42::needless_pkg {
    #[lint::skip(needless_visibility)]
    package fun skipped_needless_pkg(): u64 { 1 }

    public fun caller(): u64 {
        skipped_needless_pkg()
    }
}

// Suppression of needless_visibility for no-friends case
module 0x42::no_friends {
    #[lint::skip(needless_visibility)]
    friend fun skipped_no_friends(): u64 { 1 }

    #[deprecated]
    friend fun deprecated_no_friends(): u64 { 2 }

    #[view]
    friend fun view_no_friends(): u64 { 3 }

    public fun caller(): u64 {
        skipped_no_friends() + view_no_friends()
    }
}
