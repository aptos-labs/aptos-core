module NamedAddr::Detector {
    public fun func1(x: u64) {
        let _b = (x as u128);
        let _b = (x as u64); // <Issue:3>
    }
}