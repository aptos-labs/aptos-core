module 0x42::test {
    fun testZ() {
        let x = 3;
        let y = (x += 2) * (x -= 1);
    }
}
