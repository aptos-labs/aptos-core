module 0x815::m {
    use 0x815::m;

    enum ColorMissingComma {
        RGB{red: u64, green: u64, blue: u64}
        Red // , missing
        Blue,
    }
}
