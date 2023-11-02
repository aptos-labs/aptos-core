module NamedAddr::Detector {
    const ERROR_NUM: u64 = 2;
    public fun func1(x: u64, y: u64) {
        if (x == y || x < y) {}; // should be x <= y
        if (x < y || x == y) {}; // should be x <= y
        if (x == y || x > y) {}; // should be x >= y
        if (x > y || x == y) {}; // should be x >= y
        if (x != y || x < y) {}; // same as x < y
        if (x < y || x != y) {}; // same as x < y
        if (x != y || x > y) {}; // same as x > y
        if (x > y || x != y) {}; // same as x > y

        if (x <= y) {};
        if (x >= y) {};
        if (x > y) {};
        if (x < y) {};
    }
}