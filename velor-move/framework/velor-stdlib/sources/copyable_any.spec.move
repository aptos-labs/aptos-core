spec velor_std::copyable_any {

    // -----------------------
    // Function specifications
    // -----------------------

    spec pack<T: drop + store + copy>(x: T): Any {
        use std::bcs;
        use velor_std::from_bcs;
        aborts_if false;
        pragma opaque;
        ensures result == Any {
            type_name: type_info::type_name<T>(),
            data: bcs::serialize<T>(x)
        };
        ensures [abstract] from_bcs::deserializable<T>(result.data);
    }

    spec unpack<T>(self: Any): T {
        use velor_std::from_bcs;
        include UnpackAbortsIf<T>;
        ensures result == from_bcs::deserialize<T>(self.data);
    }

    spec schema UnpackAbortsIf<T> {
        use velor_std::from_bcs;
        self: Any;
        aborts_if type_info::type_name<T>() != self.type_name;
        aborts_if !from_bcs::deserializable<T>(self.data);
    }

    spec type_name(self: &Any): &String {
        aborts_if false;
        ensures result == self.type_name;
    }
}
