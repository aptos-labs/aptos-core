module 0x42::operator_drop {

    fun equality<T>(x: T, y: T): bool {
        x == y
    }

    fun inequality<T>(x: T, y: T): bool {
        x != y
    }

}
