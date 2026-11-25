// Closures over different instantiations of one generic function, all of the same function
// type, called from the same call in `call`; each must run its own instantiation.

// RUN: publish
module 0x1::type_info {
    use std::string::String;
    public native fun type_name<T>(): String;
}
module 0x1::generic_targets_one_call_site {
    use std::string::String;

    fun name<T>(): String { 0x1::type_info::type_name<T>() }

    fun make<T>(): || String has drop { || name<T>() }

    fun call(func: || String has drop): String { func() }

    public fun run(): (String, String, String) {
        (call(make<u8>()), call(make<u64>()), call(make<u8>()))
    }
}

// RUN: execute 0x1::generic_targets_one_call_site::run
// CHECK: results: "u8", "u64", "u8"
