module NamedAddr::Detector {
    public fun func1(x: u64) { func2(x) }
    fun func2(x: u64) {  }
    fun func3(x: u64) {  } // <Issue:4>
    inline fun func4(x: u64) {  } // <Issue:4>
    entry fun func5(x: u64) {  } // <Issue:4>
}