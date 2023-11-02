module NamedAddr::Detector {
    const ERROR_NUM: u64 = 2;
    public fun func1(x: bool) {
        if (x == true) {};
        if (x == false) {};
        if (x) {};
        if (!x) {};
    }
}