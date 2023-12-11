module NamedAddr::Detector {
    const ERROR_NUM: u64 = 2;
    public fun func1(x: u64, y: u64) {
        let z = 2;

        if (x == y) {
            if(x > y){
                if(x < z){};
                if (x == y) {};
            }
        } else if (z < y) {
        } else {};

        if(x <= y) {};

        if(x <= y) {};

        if(y >= x) {};

    }

}