//# publish
module 0x42::map_opt {
    use std::option;
    /// Maps the content of an option
    public inline fun map<Element, OtherElement>(t: option::Option<Element>, f: |Element|OtherElement): option::Option<OtherElement> {
        if (option::is_some(&t)) {
            option::some(f(option::extract(&mut t)))
        } else {
            option::none()
        }
    }

}

//# publish
module 0x42::Test {
    use std::option;
    use 0x42::map_opt;

    public fun test(): u64 {
        let t = option::some(1);
        let x = map_opt::map(t, |e| e + 1);
        option::extract(&mut x)
    }
}

//# run 0x42::Test::test
