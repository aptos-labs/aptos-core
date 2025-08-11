script {
    fun f( ) {
        let v1: || (
            |
                || has copy+drop,
                || has copy+drop,
            | has copy+drop
        ) has copy+drop
        =
        || {
            |
                v2: || has copy+drop,
                v3: || has copy+drop,
            |
            { }
        };
        let v4: |bool| has copy+drop =
            |v5: bool| {
                    *(&mut (true)) = v5;
            };
    }
}
