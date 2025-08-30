module 0xc0ffee::m {
    fun func1_warn(){
        func1_warn();
    }
    fun func1_no_warn(x: u64){
        if (x % 2 == 0){
            func1_no_warn(x);
        }
    }

    fun func2_warn(){
        func3_warn();
    }
    fun func3_warn(){
        func2_warn();
    }

    fun func2_no_warn(x: u64){
        if (x % 2 == 0){
            func3_no_warn(x);
        }
    }
    fun func3_no_warn(x: u64){
        if (x % 2 == 0){
            func2_no_warn(x);
        }
    }

    fun func4_warn(){
        func5_warn();
    }
    fun func5_warn(){
        func6_warn();
    }
    fun func6_warn(){
        func7_warn();
    }
    fun func7_warn(){
        func4_warn();
    }

    fun func4_no_warn(x: u64){
        if (x % 2 == 0){
            func5_no_warn(x);
        }
    }
    fun func5_no_warn(x: u64){
        func6_no_warn(x);
    }
    fun func6_no_warn(x: u64){
        func7_no_warn(x);
    }
    fun func7_no_warn(x: u64){
        func4_no_warn(x);
    }

    fun func8_warn(x: u64){
        if (x % 2 == 0){
            func9_warn(x);
        }else{
            func9_warn(x);
        }
    }
    fun func9_warn(x: u64){
        func8_warn(x);
    }

    fun func10_warn(x: u64){
        loop{
            func11_warn(x);
            if (x == 0){
                break;
            };
            x -= 1;
        }
    }
    fun func11_warn(x: u64){
        func10_warn(x);
    }
    
    fun func12_no_warn(x: u64){
        loop{
            if (x == 0){
                break;
            };
            func13_no_warn(x);
            x -= 1;
        }
    }
    fun func13_no_warn(x: u64){
        func12_no_warn(x);
    }

    fun func14_warn<T1, T2>(){
        func14_warn<T1, T2>();
    }

    fun func15_warn<T1, T2>(){
        func15_warn<T2, T1>();
    }

}
