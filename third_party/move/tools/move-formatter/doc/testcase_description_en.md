# Use Case Specification
## Rules through Test-Driven Development:
1. Ensure appropriate width of 90 characters, reasonable indentation of 4 spaces, and elegant layout.

2. Provide minimal configuration, such as choice of indentation with 2 or 4 spaces.

3. Try to maintain a maximum width of 90 characters internally.

4. The most statement blocks, '{' should not be written on a separate line.

5. Leave one space between keywords like if/while and the following '(' .

6. Leave one space before and after binary operators.

7. For long block comments in a single line, refer to industry tools and avoid splitting the comment.

8. Most of the time, there should be a space between the comment and the code. Block comments after '(' and before ')' do not need to leave a space.

9. Leave a blank line between functions.

10. There is a boundary condition, if the left side of '{' is exactly 90 characters long, 
then '{' does not need to move to the next line.

11. If multiple statements are on the same line, each statement is followed by a semicolon, 
and a comment is added at the end of the last semicolon. 
The formatting result should be each statement with a semicolon on a separate line, 
with the comment located after the last statement with a semicolon. 

12. For statements in the same nested level, keep the same indentation.

13. If there are one or more blank lines between two lines in a statement block, 
they will be compressed and only one blank line will be output.

14. All the trailing whitespaces will be removed.

## Comment Types:
* Block Comment -> /**/
* Block Doc Comment -> /***/
* Line Comment -> // 
* Documentation Comment -> ///

# Classification Details
> notes: The code snippets appearing in the following description have complete examples in the tests/formatter directory.


## 1.expression
### case1:
Multiple statements with semicolons in the same line, with a comment at the end. 
The formatting result should make the comment follow the last semicolon statement.
> code snippet from tests/formatter/expr/input1.move
```rust
    let y: u64 = 100; let x: u64 = 0;// Define an unsigned 64-bit integer variable y and assign it a value of 100  
```
Formatted result:
> code snippet from tests/formatter/expr/input1.move.fmt
```rust
    let y: u64 = 100;
    let x: u64 = 0;  // Define an unsigned 64-bit integer variable y and assign it a value of 100  
```

### case2:
Formatting handling for let expressions.
> code snippet from tests/formatter/expr/input2.move
```rust
let z = if (y <= 10) y = y + 1 else y = 10;  
```

### case3:
Formatting handling for combined if/else statements in the same line, which is too long and will be split into two lines after formatting.
> code snippet from tests/formatter/expr/input3.move
```rust
let z = if (y /*condition check*/ <= /*less than or equal to*/ 10) y = /*assignment*/ y + /*increment*/ 1 else y = /*assignment*/ 10;  
```

### case4:
Formatting handling for combined let and if/else statements with mixed complex comments.
> code snippet from tests/formatter/expr/input4.move
```rust
    let /*comment*/z/*comment*/ = if/*comment*/ (/*comment*/y <= /*comment*/10/*comment*/) { // If y is less than or equal to 10  
        y = y + 1; // Increment y by 1  
    }/*comment*/ else /*comment*/{  
        y = 10; // Otherwise, set y to 10  
    };  
```

### case5:
Formatting handling for arithmetic expressions (addition, subtraction, multiplication, division, etc.). 
This case involves formatting the layout of arithmetic operators and operands in an expression.

## 2.function
### case1:
The function name is very long, and the formatting program should ensure that it does not break the function name
> code snippet from tests/formatter/fun/input1.move
```rust
public fun test_long_fun_name_lllllllllllllllllllllllllllllllllllllllllllllllllllllllllllllllllllllllllllllllll(v: u64): SomeOtherStruct { SomeOtherStruct { some_field: v } }
```

### case2:
Two function blocks closely adhere to each other without any blank lines
> code snippet from tests/formatter/fun/input2.move
```rust
  // test two fun Close together without any blank lines
  public fun test_long_fun_name_lllllllllllllllllllllllllllllllllllllllllllllllllllllllllllllllllllllllllllllllll(v: u64): SomeOtherStruct{
    SomeOtherStruct{some_field: v}
  }
  public fun multi_arg(p1: u64, p2: u64): u64{
    p1 + p2
  }
```

### case3:
Two function blocks closely adhere to each other, with line or block comment in the middle
> code snippet from tests/formatter/fun/input3.move
```rust
public fun multi_arg22(p1: u64, p2: u64): u64{
    p1 + p2
  }/* test two fun Close together without any blank lines, and here is a BlockComment */public fun multi_arg22(p1: u64, p2: u64): u64{
    p1 + p2
  }
```

### case4:
There are comments before and after the function return type
> code snippet from tests/formatter/fun/input4.move
```rust
  public fun multi_arg(p1:u64,p2:u64):/* test comment locate before return type */u64{
    p1 + p2
  }

  public fun multi_arg(p1:u64,p2:u64):u64/* test comment locate after return type */{
    p1 + p2
  }
```

### case5:
The function header with resource access specifiers
> code snippet from tests/formatter/fun/input5.move
```rust
fun f_multiple() acquires R reads R writes T, S reads G<u64> {
}
```

### case6:
The comments like "//#publish", "//#[test]", "//#run"

### case7:
With generic functions (functions taking generic parameters with ability constraints)
> code snippet from tests/formatter/fun/input7.move
```rust
    public fun create_box(value: u64): Box<u64> {
        Box<u64>{ value }
    }    public fun value<T: copy>(box: &Box<T>): T acquires SomeStruct{
        *&box.value
    }
```

### case8:
Many blank lines between functions

### case9:
Multiple blank lines after module begins and before it ends

### case10:
There are blank lines at the top of a file before any code begins.

### case11:
A case for fun with many more args, so that it overflows the 90 limit, into multiple lines
> code snippet from tests/formatter/fun/input11.move
```rust
public fun multi_arg(p1:u64,p2:u64,p3:u64,p4:u64,p5:u64,p6:u64,       p7:u64,p8:u64,p9:u64,p10:u64,p11:u64,p12:u64,p13:u64,p14:u64):u64
```

## 3.lambda
### case1:
The function parameter is a lambda expression
> code snippet from tests/formatter/lambda/input1.move
```rust
public inline fun inline_apply1(f: |u64|u64, b: u64) : u64 {  
```

### case2:
Lambda is called in the while loop body
> code snippet from tests/formatter/lambda/input2.move
```rust
    public inline fun foreach<T>(v: &vector<T>, action: |&T|) {  
        // Loop through the vector and apply the action to each element  
        let i = 0;  
        while (i < vector::length(v)) {  
            action(vector::borrow(v, i));  
            i = i + 1;  
        }  
    }  
```

### case3:
Calling lambda expressions in the function body
> code snippet from tests/formatter/lambda/input3.move
```rust
        // Apply a lambda function to each element of the vector and update the product variable  
        foreach(&v, |e| product = LambdaTest1::inline_mul(product, *e));  
```

### case4:
Lambda nested calls
> code snippet from tests/formatter/lambda/input4.move
```rust
    // Public inline function with comments for parameters and return value  
    public inline fun inline_apply2(/* function g */ g: |u64|u64, /* value c */ c: u64) /* returns u64 */ : u64 {  
        // Apply the lambda function g to the result of applying another lambda function to c, and add 2 to the result  
        LambdaTest1::inline_apply1(|z|z, g(LambdaTest1::inline_mul(c, LambdaTest1::inline_apply(|x|x, 3)))) + 2  
    }  
```

### case5:
comment accompany the appearance of lambda
> code snippet from tests/formatter/lambda/input5.move
```rust
    // Public inline function with comments for each parameter and the return value  
    public inline fun inline_apply3(/* lambda function g */ g: |u64|u64, /* value c */ c: u64) /* returns u64 */ : u64 {  
        // Apply the lambda function g to the result of applying another lambda function to c, multiply the result by 3, and add 4 to the result  
        LambdaTest1::inline_apply1(g, LambdaTest1::inline_mul(c, LambdaTest1::inline_apply(|x| { LambdaTest1::inline_apply(|y|y, x) }, 3))) + 4  
    }
```

### case6:
A complex expression with function calls, binops, and parenthesis that spills over 90 columns

### case7:
A test case where we have multiple statements in a lambda's body (and one where a lambda's body contains a call to an inline function, 
one of whose arguments is a lambda, so: lambda within a lambda)


## 4.list
### case1:
A two-dimensional array with elements arranged vertically and a line of comment before each element
> code snippet from tests/formatter/list/input1.move
```rust
    // Vectorize fee store tier parameters  
    let integrator_fee_store_tiers = vector[  
        // Tier 0 parameters  
        vector[FEE_SHARE_DIVISOR_0,  
               TIER_ACTIVATION_FEE_0,  
               WITHDRAWAL_FEE_0],  
        // Tier 1 parameters  
        vector[FEE_SHARE_DIVISOR_1,  
               TIER_ACTIVATION_FEE_1,  
               WITHDRAWAL_FEE_1],  
        // Tier 2 parameters  
        vector[FEE_SHARE_DIVISOR_2,  
               TIER_ACTIVATION_FEE_2,  
               WITHDRAWAL_FEE_2],  
        // Tier 3 parameters  
        vector[FEE_SHARE_DIVISOR_3,  
               TIER_ACTIVATION_FEE_3,  
               WITHDRAWAL_FEE_3],  
        // Tier 4 parameters  
        vector[FEE_SHARE_DIVISOR_4,  
               TIER_ACTIVATION_FEE_4,  
               WITHDRAWAL_FEE_4],  
        // Tier 5 parameters  
        vector[FEE_SHARE_DIVISOR_5,  
               TIER_ACTIVATION_FEE_5,  
               WITHDRAWAL_FEE_5],  
        // Tier 6 parameters  
        vector[FEE_SHARE_DIVISOR_6,  
               TIER_ACTIVATION_FEE_6,  
               WITHDRAWAL_FEE_6]];  
```

### case2:
Each element of the array is placed on the same line, and there may be block comment before and after each element
> code snippet from tests/formatter/list/input2.move
```rust
    /** Vectorize fee store tier parameters */  
    let integrator_fee_store_tiers = vector[/** Tier 0 parameters */ vector[FEE_SHARE_DIVISOR_0, /** Activation fee for tier 0 */ TIER_ACTIVATION_FEE_0, /** Withdrawal fee for tier 0 */ WITHDRAWAL_FEE_0], /** Tier 1 parameters */ vector[FEE_SHARE_DIVISOR_1, TIER_ACTIVATION_FEE_1, WITHDRAWAL_FEE_1], /** Tier 2 parameters */ vector[FEE_SHARE_DIVISOR_2, TIER_ACTIVATION_FEE_2, WITHDRAWAL_FEE_2], /** Tier 3 parameters */ vector[FEE_SHARE_DIVISOR_3, TIER_ACTIVATION_FEE_3, WITHDRAWAL_FEE_3], /** Tier 4 parameters */ vector[FEE_SHARE_DIVISOR_4, TIER_ACTIVATION_FEE_4, WITHDRAWAL_FEE_4], /** Tier 5 parameters */ vector[FEE_SHARE_DIVISOR_5, TIER_ACTIVATION_FEE_5, WITHDRAWAL_FEE_5], /** Tier 6 parameters */ vector[FEE_SHARE_DIVISOR_6, TIER_ACTIVATION_FEE_6, WITHDRAWAL_FEE_6]];  

```

### case3:
A two-dimensional array with elements arranged vertically, with comment at the end of each row of elements
> code snippet from tests/formatter/list/input3.move
```rust
    // Define a vector of fee store tiers as a 2D vector  
    let integrator_fee_store_tiers = vector[vector[FEE_SHARE_DIVISOR_0, // Fee share divisor for tier 0  
                                                  TIER_ACTIVATION_FEE_0, // Activation fee for tier 0  
                                                  WITHDRAWAL_FEE_0],      // Withdrawal fee for tier 0  
                                            vector[FEE_SHARE_DIVISOR_1, // Fee share divisor for tier 1  
                                                  TIER_ACTIVATION_FEE_1, // Activation fee for tier 1  
                                                  WITHDRAWAL_FEE_1],      // Withdrawal fee for tier 1  
                                            vector[FEE_SHARE__DIVISOR__2, FEE__SHARE__DIVISOR__2, FEE__SHARE__DIVISOR__2]]; // ... and so on for other tiers  
```

### case4:
A two-dimensional array with elements arranged vertically, with block comment before and after each row of elements
> code snippet from tests/formatter/list/input4.move
```rust
   // Vectorize fee store tier parameters  
    let integrator_fee_store_tiers = vector[  
        // Tier 0 parameters  
        vector[//comment
        FEE_SHARE_DIVISOR_0,  
               TIER_ACTIVATION_FEE_0,  
               WITHDRAWAL_FEE_0],  
        // Tier 1 parameters  
        vector[FEE_SHARE_DIVISOR_1,  
        //comment
               TIER_ACTIVATION_FEE_1,  
               WITHDRAWAL_FEE_1],  
        // Tier 2 parameters  
        vector[FEE_SHARE_DIVISOR_2, 
                //comment 
               TIER_ACTIVATION_FEE_2,  
               WITHDRAWAL_FEE_2],  
        // Tier 3 parameters  
        vector[/*comment*/FEE_SHARE_DIVISOR_3,  
               TIER_ACTIVATION_FEE_3/*comment*/,  
               WITHDRAWAL_FEE_3],  
        // Tier 4 parameters  
        vector[FEE_SHARE_DIVISOR_4,  
               TIER_ACTIVATION_FEE_4, /*comment*/ 
               WITHDRAWAL_FEE_4],  
        // Tier 5 parameters  
        vector[FEE_SHARE_DIVISOR_5,  
               /*comment*/TIER_ACTIVATION_FEE_5,  
               WITHDRAWAL_FEE_5],  
        // Tier 6 parameters  
        vector[FEE_SHARE_DIVISOR_6,  
               TIER_ACTIVATION_FEE_6,  
               WITHDRAWAL_FEE_6]];  /*comment*/
```

### case5:
A two-dimensional array with elements arranged vertically and some rows with empty spaces between them
> code snippet from tests/formatter/list/input5.move
```rust
    let integrator_fee_store_tiers = vector[
        vector[FEE_SHARE_DIVISOR_0, TIER_ACTIVATION_FEE_0, WITHDRAWAL_FEE_0],
        
        vector[FEE_SHARE_DIVISOR_1, TIER_ACTIVATION_FEE_1, WITHDRAWAL_FEE_1],
        // ...
        vector[FEE_SHARE_DIVISOR_N, TIER_ACTIVATION_FEE_N, WITHDRAWAL_FEE_N]
    ];
```

## 5.spec_fun
### case1:
has {'apply', 'exists', 'global'}
> code snippet from tests/formatter/spec_fun/input1.move
```rust
apply CapAbortsIf to *<Feature> except spec_delegates;
// ...
exists<CapState<Feature>>(addr)
// ...
global<CapState<Feature>>(addr).delegates
```

### case2:
No blank lines between two functions
> code snippet from tests/formatter/spec_fun/input2.move
```rust
spec fun spec_table_len<K, V>(t: TableWithLength<K, V>): u64 {
            table_with_length::spec_len(t)
        }
        spec fun spec_table_contains<K, V>(t: TableWithLength<K, V>, k: K): bool {
            table_with_length::spec_contains(t, k)
        }
```

### case3:
comment between two functions
> code snippet from tests/formatter/spec_fun/input3.move
```rust
spec fun spec_table_len<K, V>(t: TableWithLength<K, V>): u64 {
            table_with_length::spec_len(t)
        }
// comment
        spec fun spec_table_contains<K, V>(t: TableWithLength<K, V>, k: K): bool {
            table_with_length::spec_contains(t, k)
        }
```

### case4:
fun name too long
> code snippet from tests/formatter/spec_fun/input4.move
```rust
spec singletonlllllllllllllllllllllllllllllllllllllllllllllllllllllllllllllllllllllllllllll<T: store>(element: T, bucket_size: u64): BigVector<T> {
    ensures length(result) == 1;
    ensures result.bucket_size == bucket_size;
}
```

### case5:
all kinds of comment in spec fun
> code snippet from tests/formatter/spec_fun/input5.move
```rust
    spec fun spec_at<T>(v: BigVector<T>/*comment*/, i: u64): T {
        let bucket = i / v.bucket_size;//comment
        //comment
        let idx =/*comment*/ i % v.bucket_size;
        /// comment
        let v = table_with_length::spec_get(v.buckets, /*comment*/bucket);
        /*comment*/
        v[idx]
    }
```

## 6.spec module
### case1:
has {'pragma', 'aborts_if', 'ensures'}
> code snippet from tests/formatter/spec_module/input1.move
```rust
pragma opaque;
        aborts_if [abstract] false;
        ensures [abstract] result == spec_internal_is_char_boundary(v, i);
```

### case2:
has {'native fun'}
> code snippet from tests/formatter/spec_module/input2.move
```rust
native fun serialize<MoveValue>(v: &MoveValue): vector<u8>;
```

### case3:
has {'requires'}
> code snippet from tests/formatter/spec_module/input3.move
```rust
requires exists<coin::CoinInfo<AptosCoin>>(@aptos_framework);
```

### case4:
There is only one comment and no code in the module block
> code snippet from tests/formatter/spec_module/input4.move
```rust
spec module {/*comment*/} // switch to module documentation context
```

### case5:
has {'use', 'include'}
> code snippet from tests/formatter/spec_module/input5.move
```rust
 use aptos_framework::staking_config;
        pragma verify_duration_estimate = 120; // TODO: set because of timeout (property proved)
    requires chain_status::is_operating();   
        include transaction_fee::RequiresCollectedFeesPerValueLeqBlockAptosSupply;
```

## 7.spec_struct
### case1:
has ability{'copy', 'drop', 'store'}
> code snippet from tests/formatter/spec_struct/input1.move
```rust
struct String has copy, drop, store {
```

### case2:
has {'invariant'}
> code snippet from tests/formatter/spec_struct/input2.move
```rust
invariant is_valid_char(byte);
```

### case3:
has ability{'copy', 'drop', 'store'} with comment
> code snippet from tests/formatter/spec_struct/input3.move
```rust
struct String has /*comment*/copy, drop/*comment*/, store /*comment*/{
       // comment
       bytes: vector<u8>,// comment

   }
```

### case4:
Struct field has comments
> code snippet from tests/formatter/spec_struct/input4.move
```rust
   /// An ASCII character.
   struct Char has copy, drop, store {
    // comment
       byte: u8,
   }

```

### case5:
Struct ability written on multiple lines
> code snippet from tests/formatter/spec_struct/input5.move
```rust
   /// An ASCII character.
   struct Char has copy,/*comment*/ 
    /*comment*/drop, 
    // comment
    store {
    // comment
       byte: u8,
   }
   spec Char {
    // comment
       invariant is_valid_char(byte);//comment
   }
```

## 8.struct
### case1:
At the definition of the structure, each field has comment at different positions
> code snippet from tests/formatter/struct/input1.move
```rust
    struct TestStruct1 {  
        // This is field1 comment  
        field1: u64,  
        field2: bool,  
    } 
    struct TestStruct2 { // This is a comment before struct definition 
        field1: u64, // This is a comment for field1  
        field2: bool, // This is a comment for field2  
    }  // This is a comment after struct definition 
    struct TestStruct4<T> {  
        // This is a comment before complex field  
        field: vector<T>, // This is a comment after complex field  
    }  
```

### case2:
Structured variable with prototype as function parameter
> code snippet from tests/formatter/struct/input2.move
```rust
    // Function using the struct  
    fun use_complex_struct1(s: ComplexStruct1<u64, bool>) {  
        // Function comment  
    }  
```

### case3:
Structural definition with prototype
> code snippet from tests/formatter/struct/input3.move
```rust
    // Struct with comments in various positions  
    struct ComplexStruct1<T, U> {  
        // Field 1 comment  
        field1: vector<U>, // Trailing comment for field1  
        // Field 2 comment  
        field2: bool,  
        // Field 3 comment  
        field3: /* Pre-comment */ SomeOtherStruct<T> /* Post-comment */,  
    } /* Struct footer comment */  
```

### case4:
There are blank lines or line comments between structure fields
> code snippet from tests/formatter/struct/input4.move
```rust
    // Struct with nested comments and complex types  
    struct ComplexStruct2<T, U> {  
        
        field1: /* Pre-comment */ vector<T> /* Inline comment */,  
        
        field2: /* Comment before complex type */ SomeGenericStruct<U> /* Comment after complex type */,  
        
        field3: /* Pre-comment */ optional<bool> /* Post-comment */,  
    } // Struct footer comment  
```

### case5:
has ability as {copy, drop, store}
> code snippet from tests/formatter/struct/input5.move
```rust
    // Integrator fee store tier parameters for a given tier.
    struct IntegratorFeeStoreTierParameters has drop, store {
        // Nominal amount divisor for taker quote coin fee.
        fee_share_divisor: u64,
    }
```

## 9.tuple
### case1:
Function returns an empty tuple
> code snippet from tests/formatter/tuple/input1.move
```rust
// when no return type is provided, it is assumed to be `()`
fun returs_unit_1() { }

// there is an implicit () value in empty expression blocks
fun returs_unit_2(): () { }

// explicit version of `returs_unit_1` and `returs_unit_2`
fun returs_unit_3(): () { () }
```

### case2:
Function tuple with comment for different elements
> code snippet from tests/formatter/tuple/input2.move
```rust
        fun returns_3_values(): (u64, bool, address) {
            // comment
            (0, /*comment*/false/*comment*/, @0x42)// comment
        }
        fun returns_4_values(x: &u64): (&u64, u8, u128, vector<u8>) {            
            (x/*comment*/, 0/*comment*/, /*comment*/1/*comment*/, /*comment*/b"foobar"/*comment*/)
        }
```

### case3:
Elements in tuple brackets, each occupying one line and accompanied by comments
> code snippet from tests/formatter/tuple/input3.move
```rust
        fun returns_3_values(): (u64, bool, address) {
            // comment
            (
                // comment
                0, 
            /*comment*/false/*comment*/, 
            @0x42)// comment
        }
```

### case4:
Tuple is defined and assigned values by various types of expressions
> code snippet from tests/formatter/tuple/input3.move
```rust
            // This line is an example of a unit value being assigned to a variable  
            let () = ();

            // This line is an example of a tuple with multiple types being assigned to variables a, b, c, and d  
            let (a, b, c, d) = (@0x0, 0, false, b"");  
  
            // Reassignment of unit value  
            () = ();  
              
            // Conditional reassignment of tuple values x and y  
            (x, y) = if (cond) (1, 2) else (3, 4);  
              
            // Reassignment of tuple values a, b, c, and d  
            (a, b, c, d) = (@0x1, 1, true, b"1");
```

### case5:
Tuples are defined by various types of expressions, and elements are annotated around them.
> code snippet from tests/formatter/tuple/input5.move
```rust
        /**  
         * This function returns a tuple containing four values: a reference to a u64 value,  
         * a u8 value, a u128 value, and a vector of u8 values.  
         */  
        fun returns_4_values(x: &u64): /*(&u64, u8, u128, vector<u8>)*/ (&u64, u8, u128, vector<u8>) { (x, /*comment*/0, 1, b"foobar") }  
          
        /**  
         * This function demonstrates various examples using tuples.  
         * It includes assignments to tuple variables and reassignments using conditional statements.  
         */  
        fun examples(cond: bool) {
            // Assignment of tuple values to variables x and y  
            let (x, y): /*(u8, u64)*/ (u8, u64) = (0, /*comment*/1);
        }
```

## 10.use
### case1:
A list consisting of multiple items, with comments after the items
> code snippet from tests/formatter/use/input1.move
```rust
    use aptos_framework::aptos_coin::AptosCoin;
        use aptos_framework::coin::{Self, Coin}/* use */;
                use aptos_std::type_info::{Self/* use_item after */, TypeInfo};
            use econia::resource_account;
        use econia::tablist::{Self, Tablist/* use_item after */};
            use std::signer::address_of;
    use std::vector;
```

### case2:
A list consisting of multiple items, with comments before the items
> code snippet from tests/formatter/use/input2.move
```rust
    use aptos_framework::aptos_coin::AptosCoin;
        use aptos_framework::coin::{Self, Coin};
                use aptos_std::type_info::{/* use_item before */Self, TypeInfo};
            use econia::resource_account;
        use econia::tablist::{Self, /* use_item before */Tablist};
            use std::signer::address_of;
    use std::vector;
```

### case3:
Use items one by one, with block comments on each line
> code snippet from tests/formatter/use/input3.move
```rust
        use aptos_std::type_info::{
            /* use_item before */Self, 
            TypeInfo
        };
    use aptos_framework::coin::{
        Self, 
        /* use_item before */Coin};
```

### case4:
Use items one by one, with inline comments on each line
> code snippet from tests/formatter/use/input4.move
```rust
    use aptos_std::type_info::{
        // use_item
        Self, 
        TypeInfo
    };
use aptos_framework::coin::{
    Self, 
    // use_item
    Coin};
```

### case5:
Multiple blank lines between use statements
> code snippet from tests/formatter/use/input5.move
```rust
    // Multiple blank lines between statements
        use aptos_std::type_info::{
            /* use_item before */Self, 

            TypeInfo
        };



    use aptos_framework::coin::{
        Self, 

        /* use_item before */Coin};
  
```

### case6:
The `use` statements overflows the 90 column limit; Coallescable `use` statements

## 11.other
### case1:
`const` declarations, complex and multiple attributes

### case2:
`friend` declarations, `friend` fun

### case3:
chained access that exceed column limit
