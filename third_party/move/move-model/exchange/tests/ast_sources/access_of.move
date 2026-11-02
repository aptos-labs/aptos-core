module 0x42::access_of {
    struct Counter has key { value: u64 }
    struct Config has key { active: bool }

    struct Holder has key {
        f: |address| has copy + store + drop,
    }
    spec Holder {
        modifies_of<f>(a: address) Counter[a];
    }

    fun apply(f: |address|, x: address) {
        f(x)
    }
    spec apply {
        reads_of<f> Config;
        modifies_of<f>(a: address) Counter[a];
    }

    fun apply_any(f: |u64| u64, x: u64): u64 {
        f(x)
    }
    spec apply_any {
        modifies_of<f> *;
    }
}
