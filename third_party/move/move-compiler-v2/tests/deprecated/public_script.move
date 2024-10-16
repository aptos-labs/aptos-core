module 0x1::Test {
    public(script) fun main() {
        // Previously, deprecation plus an error led to a compiler assert.  Make sure that doesn't come back.
        let _addr:address = @Test;
    }
}
