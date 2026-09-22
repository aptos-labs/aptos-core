module 0x47::vector_literals {
    public fun pair(a: u64, b: u64): vector<u64> {
        vector[a, b]
    }

    public fun nested(n: u64): vector<vector<u64>> {
        vector[vector[n], vector[n, n], vector[]]
    }

    public fun deep(n: u64): vector<vector<vector<u64>>> {
        vector[vector[vector[n]]]
    }

    public fun empty(): vector<u64> {
        vector[]
    }
}
