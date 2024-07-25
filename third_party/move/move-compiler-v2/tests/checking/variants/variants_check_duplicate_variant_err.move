module 0x815::m {

    enum ColorDuplicateVariant {
        RGB{red: u64, green: u64, blue: u64},
        Red,
        Blue,
        RGB{red: u64, green: u64, blue: u64},
    }
}
