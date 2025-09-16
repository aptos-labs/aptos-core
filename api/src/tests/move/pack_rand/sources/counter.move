module addr::rand {
    use 0x1::randomness::u64_integer;


    #[randomness]
    entry fun rand_integer() {
        let _ = u64_integer();
    }
}
