module 0x42::test {
    fun contains(v: &vector<u64>, p: |u64|bool): bool {
        p(v[0])
    }
    spec contains {
        ensures result == p(v[0]);
    }

    fun using(): bool {
        let v = vector[1];
        contains(&v, |x| x == 1)
    }
    spec using {
        ensures result;
    }
}
