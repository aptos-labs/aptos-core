//# publish
module 0x42::test {
    fun for(i: u32, j: u32, k: u32) : u32 {
        i + j + k
    }
    public fun for_user() : u32 {
        let (i, j, k) = (3, 4, 5);
        let x = for(i, j, k);
        x
    }
}

//# run 0x42::test::for_user
