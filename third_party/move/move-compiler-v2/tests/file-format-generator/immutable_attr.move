// Tests that #[immutable] is emitted as a FunctionAttribute in the bytecode
// on functions of any visibility (public, private, friend).
module 0x42::immutable_attr {
    // public: carries [immutable, persistent]
    #[immutable]
    public fun locked_pub(): u64 { 99 }

    // private: carries [immutable, persistent] -- #[immutable] implies #[persistent]
    #[immutable]
    fun locked_priv(): u64 { 42 }

    // A plain public function without the attribute.
    public fun unlocked(): u64 { 1 }
}
