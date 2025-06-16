/**
/*
这个也是未闭合的文档注释
*/

module 0xc0ffee::m {
    // 这个是代码注释
    /// 这个也是文档注释
    ////
    // This is a mixed comment with ascii
    // and 中文注释
    struct Test1 has copy, drop {
        a: u64, // this is purely english comment
        b: u64 // this is a comment with wired symbols é ñ ü ß œ ☀ ★ ☯ €
    }
}
