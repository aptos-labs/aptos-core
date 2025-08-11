script {
    fun f( ) {
        let v1: || (
            |
                || has copy+drop,
            | has copy+drop
        ) has copy+drop
        =
        || {
            |
                v2: || has copy+drop,
            |
            { }
        };
        let v3: |bool| has copy+drop =
            |v4: bool| {
                    *(&mut (true)) = v4;
            };
    }
}
