spec aptos_std::copyable_any {

    // -----------------------
    // Function specifications
    // -----------------------

    spec pack<T: drop + store + copy>(x: T): Any {
        use std::bcs;
        use aptos_std::from_bcs;
        aborts_if false;
        pragma opaque;
        ensures result == Any {
            type_name: type_info::type_name<T>(),
            data: bcs::serialize<T>(x)
        };
        ensures [abstract] from_bcs::deserializable<T>(result.data);
    }

    spec unpack<T>(x: Any): T {
        use aptos_std::from_bcs;
        include UnpackAbortsIf<T>;
        ensures result == from_bcs::deserialize<T>(x.data);
    }

    spec schema UnpackAbortsIf<T> {
        use aptos_std::from_bcs;
        x: Any;
        aborts_if type_info::type_name<T>() != x.type_name;
        aborts_if !from_bcs::deserializable<T>(x.data);
    }

    spec type_name(x: &Any): &String {
        aborts_if false;
        ensures result == x.type_name;
    }
}
