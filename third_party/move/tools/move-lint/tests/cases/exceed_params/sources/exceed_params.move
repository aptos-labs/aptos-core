module NamedAddr::Detector {
    //Assume we have 5 fields to test the lint
    struct ExampleStruct<T1, T2, T3, T4, T5> has store {
        field1: T1,
        field2: T2,
        field3: T3,
        field4: T4,
        field5: T5,
    }
    public fun create_example_struct<T1, T2, T3, T4, T5>(
        field1: T1, 
        field2: T2, 
        field3: T3, 
        field4: T4, 
        field5: T5,
    ): ExampleStruct<T1, T2, T3, T4, T5> {
        ExampleStruct {
            field1,
            field2,
            field3,
            field4,
            field5,
        }
    }
}
