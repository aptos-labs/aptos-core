module NamedAddr::Detector {
    public fun func1(x: u64, y: u64, z:u64) {
        let a = x * y / z;
        let b = x / z * y; // <Issue:7>
        //Multiplication followed by Division (Expected Warnings)
        let _c = x * (y / z); // <Issue:9>
        let _d = x * y / (z * 2); // <Issue:10>
        let _e = (x * 2) / y; // <Issue:11>

        //Parentheses and Nested Expressions (Expected Warnings)
        let _f = (x * y) / (z + 1); // <Issue:12>
        let _g = (x * (y + 1)) / z; // <Issue:13>
        let _h = x * (y / (z - 1)); // <Issue:14>

        let _i = (x * y) / z * (a / b); // <Issue:15>
        let _j = x * y * a / (z + b); // <Issue:16>
        let _k = x * (y / (z + a)) / b; // <Issue:17>
        let _l = x * (y / (z * a)) / (b + 1); // <Issue:18>
    }
}