module NamedAddr::Detector {
    use std::string::{String, utf8};
    use std::signer;
    
    struct Person has key {
        name: String,
        age: u64,
    }

    // Correct getter method (method name matches field name)
    public fun get_name(user_addr: address): String acquires Person {
        return borrow_global<Person>(user_addr).name
    }

    // Incorrect getter method (method name does not match field name)
    public fun get_age(user_addr: address): String acquires Person {
       return borrow_global<Person>(user_addr).name
    }

    // Another struct for more comprehensive testing
    struct Address has key {
        street: String,
        city: String,
    }

    // Correct getter method
    public fun get_street(user_addr: address): String acquires Address {
       return borrow_global<Address>(user_addr).street
    }

    // Incorrect getter method
    public fun get_city(user_addr: address): u64 {
        // This should return a string, corresponding to the 'city' field
        return 123
    }

}
