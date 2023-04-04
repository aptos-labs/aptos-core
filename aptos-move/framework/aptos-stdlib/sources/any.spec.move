spec aptos_std::any {

    // -----------------------
    // Function specifications
    // -----------------------

    spec pack<T: drop + store>(x: T): Any {
        use std::bcs;
        aborts_if false;
        ensures result == Any {
            type_name: type_info::type_name<T>(),
            data: bcs::serialize<T>(x)
        };
    }

    spec unpack<T>(x: Any): T {
        use aptos_std::from_bcs;
        aborts_if type_info::type_name<T>() != x.type_name;
        aborts_if !from_bcs::deserializable<T>(x.data);
        ensures result == from_bcs::deserialize<T>(x.data);
    }

    spec type_name(x: &Any): &String {
        aborts_if false;
        ensures result == x.type_name;
    }
}
