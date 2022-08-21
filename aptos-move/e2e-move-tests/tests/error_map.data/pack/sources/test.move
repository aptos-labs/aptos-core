module 0xcafe::test {

    /// This error is raised because it wants to.
    const ESOME_ERROR: u64 = 0x1;


    /// This error is often raised as well.
    const ESOME_OTHER_ERROR: u64 = 0x10002; // We also support category given here.


    public entry fun entry(_s: &signer, value: bool) {
        if (value) {
            abort 0x20001
        } else {
            abort 0x10002
        }
    }
}
