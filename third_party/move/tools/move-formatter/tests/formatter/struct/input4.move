module complex_module {  
    
    // Struct with nested comments and complex types  
    struct ComplexStruct2<T, U> {  
        
        field1: /* Pre-comment */ vector<T> /* Inline comment */,  
        
        field2: /* Comment before complex type */ SomeGenericStruct<U> /* Comment after complex type */,  
        
        field3: /* Pre-comment */ optional<bool> /* Post-comment */,  
    } // Struct footer comment  

    // Struct with various comment styles and positions  
    struct ComplexStruct3 {  
        // Field 1 comment (single-line)  
                field1: u64, // Inline comment (line 1)
                            // continued on line 2.
        /* Field 2 comment (multi-line) */  
        field2: /* Inline comment (multi-line) */ bool, // Trailing comment (single-line)  
    } // Struct footer comment (single-line)    
}
