 module 0xc0ffee::m {
    //Note: This test is actually a false negative. An ideal linter would be
    //able to detect that the sequence from line 4 to line 18 has no effect and
    //flag it, but it's not possible because linters don't see the AST as it is
    //in the source code. If this is fixed in the future and this test fails
    //because of that, don't treat it as a regression.
    public fun test1_no_warn(n: u64){
        {
            let x = 6;
            let n2 = n;
            loop{
                if (n == 0){
                    break;
                };
                x += 1;
                n2 -= 1;
            };
        }
    }
}
