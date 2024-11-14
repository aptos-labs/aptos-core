script {
    // Note: this constant can be replaced in compiled script to make it hash to a different value.
    const SENDER: address = @0x1;

    fun main(sender: &signer) {
        // The idea is to to ensure that this script takes some time to be deserialized and verified, but the actual
        // execution time is small (no-op).
        if (false) {
            0xABCD::simple::loop_nop(sender, 0);
            0xABCD::simple::loop_arithmetic(sender, 0);
            0xABCD::simple::loop_bcs(sender, 0, 0);
            if (false) {
                while (true) {}
            }
        }
    }
}
