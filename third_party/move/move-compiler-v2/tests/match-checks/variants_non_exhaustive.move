module 0x815::m {

    enum Color {
        RGB{red: u64, green: u64, blue: u64},
        Red,
        Blue(),
    }

    fun t4(c: &Color) {
        match (c) { Red => abort 1, Blue => abort 2 }
    }
}
