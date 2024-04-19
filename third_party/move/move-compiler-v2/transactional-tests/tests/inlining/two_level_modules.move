//# publish
module 0x42::TwoLevelTestModule {
    public inline fun f1(x: u64): u64 {
	x * 3
    }

    public inline fun f2(x: u64): u64 {
        2 * x
    }
}

//# publish
module 0x42::TwoLevelTestMain {
    use 0x42::TwoLevelTestModule;

    public fun test(): u64 {
        TwoLevelTestModule::f1(TwoLevelTestModule::f2(3))
    }
}

//# run 0x42::TwoLevelTestMain::test
