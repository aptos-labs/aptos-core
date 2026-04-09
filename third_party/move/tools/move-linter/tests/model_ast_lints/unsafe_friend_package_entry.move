module 0x42::friend_caller {}

// Test friend entry functions
module 0x42::entry_points_friend {
    friend 0x42::friend_caller;

    // Should warn: friend entry is callable by anyone
    friend entry fun unsafe_friend_entry() {}

    // Ok: suppressed with lint::skip
    #[lint::skip(unsafe_friend_package_entry)]
    friend entry fun suppressed_friend_entry() {}

    // Ok: private entry function
    entry fun private_entry() {}

    // Ok: public entry function (fully public, no misleading restriction)
    public entry fun public_entry() {}

    // Ok: non-entry friend function (restriction is real)
    #[lint::skip(unused_function)]
    friend fun friend_non_entry() {}
}

// Test package entry functions
module 0x42::entry_points_package {
    // Should warn: package entry is callable by anyone
    package entry fun unsafe_package_entry() {}

    // Ok: suppressed with lint::skip
    #[lint::skip(unsafe_friend_package_entry)]
    package entry fun suppressed_package_entry() {}

    // Ok: non-entry package function (restriction is real)
    #[lint::skip(unused_function)]
    package fun package_non_entry() {}
}
