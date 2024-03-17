module NamedAddr::Detector {
    const ERROR_NUM: u64 = 2;
    public fun func1(x: bool) {
        if (x == true) {};
        if (x == false) {};
        if (x != true) {};
        if (x != false) {};
        if (x == true || ERROR_NUM == 2) {};
        if (x == true && x != false) {};
        if (x) {};
        if (!x) {};
        if (true == x) {};
        if (condition() == true) {};

        if ((condition() && x == true) && (!x == false)){};
        let y = (x == true);
        if (true == condition()) {};
        if (condition() == true) {};
    }

    fun condition(): bool {
        true
    }
}
