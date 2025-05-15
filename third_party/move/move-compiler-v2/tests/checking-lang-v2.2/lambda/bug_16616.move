script {
    fun f( ) {
        (
            | x: |u8| has drop | { 1u16 } // Lambda
        )(
            | y: u8 | { 1u8 & y; } // Argument
        ) >= 1u16;
    }
}
