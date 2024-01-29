## Background

A formatting tool, also known as a pretty-printer, prints the AST of the corresponding language into a beautifully formatted string.

Implementing a formatter for programming language always with a lot of work. First of all, a language AST always has a lot of variant data structure, like move expression.

```rust
pub enum Exp_ {
    Value(Value),
    // move(x)
    Move(Var),
    // copy(x)
    Copy(Var),
    // [m::]n[<t1, .., tn>]
    Name(NameAccessChain, Option<Vec<Type>>),

    // f(earg,*)
    // f!(earg,*)
    Call(NameAccessChain, bool, Option<Vec<Type>>, Spanned<Vec<Exp>>),

    // tn {f1: e1, ... , f_n: e_n }
    Pack(NameAccessChain, Option<Vec<Type>>, Vec<(Field, Exp)>),

    // vector [ e1, ..., e_n ]
    // vector<t> [e1, ..., en ]
    Vector(
        /* name loc */ Loc,
        Option<Vec<Type>>,
        Spanned<Vec<Exp>>,
    ),

    // if (eb) et else ef
    IfElse(Box<Exp>, Box<Exp>, Option<Box<Exp>>),
    // while (eb) eloop
    While(Box<Exp>, Box<Exp>),
    // loop eloop
    Loop(Box<Exp>),

    // { seq }
    Block(Sequence),
    // fun (x1, ..., xn) e
    Lambda(BindList, Box<Exp>), // spec only
    // forall/exists x1 : e1, ..., xn [{ t1, .., tk } *] [where cond]: en.
    Quant(
        QuantKind,
        BindWithRangeList,
        Vec<Vec<Exp>>,
        Option<Box<Exp>>,
        Box<Exp>,
    ), // spec only
    // (e1, ..., en)
    ExpList(Vec<Exp>),
    // ()
    Unit,
    
    ...
}
```
This is just `Expression` variants. There are also `Function`,`Module`,`Struct`,`Spec`,etc. Implement a formatter you have to deal all the data structure.


## Challenge of movefmt 
### Spec
Spec is the abbreviation for Move specification language in AST, a subset of the Move language which supports specification of the behavior of Move programs. It contains many grammar elements such as modules, type system, functions, declaration statements, quantifier expressions, and so on.

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpecApplyPattern_ {
    pub visibility: Option<Visibility>,
    pub name_pattern: Vec<SpecApplyFragment>,
    pub type_parameters: Vec<(Name, Vec<Ability>)>,
}

pub type SpecApplyPattern = Spanned<SpecApplyPattern_>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SpecApplyFragment_ {
    Wildcard,
    NamePart(Name),
}

pub type SpecApplyFragment = Spanned<SpecApplyFragment_>;

#[derive(Debug, Clone, PartialEq)]
#[allow(clippy::large_enum_variant)]
pub enum SpecBlockMember_ {
    Condition {
        kind: SpecConditionKind,
        properties: Vec<PragmaProperty>,
        exp: Exp,
        additional_exps: Vec<Exp>,
    },
    Function {
        uninterpreted: bool,
        name: FunctionName,
        signature: FunctionSignature,
        body: FunctionBody,
    },
    Variable {
        is_global: bool,
        name: Name,
        type_parameters: Vec<(Name, Vec<Ability>)>,
        type_: Type,
        init: Option<Exp>,
    },
    Let {
        name: Name,
        post_state: bool,
        def: Exp,
    },
    Update {
        lhs: Exp,
        rhs: Exp,
    },
    Include {
        properties: Vec<PragmaProperty>,
        exp: Exp,
    },
    Apply {
        exp: Exp,
        patterns: Vec<SpecApplyPattern>,
        exclusion_patterns: Vec<SpecApplyPattern>,
    },
    Pragma {
        properties: Vec<PragmaProperty>,
    },
}

pub type SpecBlockMember = Spanned<SpecBlockMember_>;

// Specification condition kind.
#[derive(PartialEq, Eq, Clone, Debug)]
pub enum SpecConditionKind_ {
    Assert,
    Assume,
    Decreases,
    AbortsIf,
    AbortsWith,
    SucceedsIf,
    Modifies,
    Emits,
    Ensures,
    Requires,
    Invariant(Vec<(Name, Vec<Ability>)>),
    InvariantUpdate(Vec<(Name, Vec<Ability>)>),
    Axiom(Vec<(Name, Vec<Ability>)>),
}
pub type SpecConditionKind = Spanned<SpecConditionKind_>;

```
It is a very complex language feature that movefmt needs to support.

### Comment
Another complex thing about formatter is `Comments`. Move programming language support three forms of comments.
* Block Comment -> /**/
* Line Comment -> // 
* Documentation Comment -> ///

`Comments` can write anywhere in source code.

In order to keep user's comments you have to keep comments in AST like below.
```rust
    // f(earg,*)
    // f!(earg,*)
    Call(NameAccessChain,
        Vec<Comment> , // keep in AST.
        bool, Option<Vec<Type>>, Spanned<Vec<Exp>>),
```
This will make things below ugly.

* AST Definition.
* Parse AST.
* All routine that accept AST.

In general We need keep a `Vec<Comment>` in every AST structure. But in the move-compiler module, after syntax analysis, comments are filtered and there is no comment information on the AST.


Is there a way to slove this puzzle?


## MOVEFMT SOLUTION
### Overview of Our Approach:
#### Two Core Structures:
```rust
struct FormatContext
struct TokenTree
```
#### Other Important Structures:
```rust
enum FormatEnv
struct FormatConfig
```
#### Comment Processing Structure:
```rust
struct CommentExtrator
struct Comment
```
#### Main Logic Structure for Rewriting:
```rust
struct Fmt
```

### Overall Logic:
#### Step 1: Extract various data structures from source code.
1).FormatConfig stores formatting rules.

2).Obtain TokenTree through lexical analysis.

3).Retrieve AST through syntactic analysis.

4).Gather comments using CommentExtrator.


#### Step 2: Traverse the TokenTree for formatting.
While traversing, Fmt considers Definition from move_compiler::parser::ast to calculate the current code block's FormatEnv. 
This helps identify if the block is a function, structure, or a block of use statements, among others.
The FormatEnv is stored within the FormatContext structure. Based on the FormatEnv, Fmt invokes different formatters like use_fmt, expr_fmt, etc.

#### Step 3: Within a specific FormatContext, perform formatting according to FormatConfig.

### Additional Details:
We devise a structure, TokenTree. This categorizes the AST returned by the parser into two groups: 
code blocks (i.e., NestedTokens enclosed by "()" or "{}") and everything else as SimpleToken.
Evidently, NestedToken is a nested structure. Ignoring the module name, any Move module essentially constitutes a NestedToken. 
Example:
```rust
module econia::incentives {  // NestedToken1  
    // ...  
    {  // NestedToken2  
        {  // NestedToken3  
            // ...  
        }  
    }  
    // ...  
}
```
There are four types of SimpleToken: "module", "econia", "::", "incentives".

The remaining main body of the module is entirely placed within NestedToken1, which also includes nested tokens like NestedToken2 and NestedToken3.

#### Rationale for Introducing TokenTree:

##### Reason 1: 
Formatting code essentially involves formatting nested code blocks within a file. Therefore, by abstracting the AST with TokenTree, we can better align with the formatting's business logic.

##### Reason 2: 
Data from TokenTree originates directly from Lexer. Since Lexer records position, type, and symbol name for each token, 
it becomes convenient to add spaces between tokens and insert line breaks where necessary. In summary, it allows for more precise control over the output format.



## Detail design
### comment parse and process
#### Data structure
```rust
pub struct Comment {
    pub start_offset: u32,
    pub content: String,
}

pub enum CommentKind {
    /// "//"
    InlineComment,
    /// "///"
    DocComment,
    /// "/**/"
    BlockComment,
}

pub struct CommentExtrator {
    pub comments: Vec<Comment>,
}
```

#### Extrator algorithm
```rust
pub enum ExtratorCommentState {
    /// init state
    Init,
    /// `/` has been seen,maybe a comment.
    OneSlash,
    /// `///` has been seen,inline comment.
    InlineComment,
    /// `/*` has been seen,block comment.
    BlockComment,
    /// in state `BlockComment`,`*` has been seen,maybe exit the `BlockComment`.
    OneStar,
    /// `"` has been seen.
    Quote,
}
```
>pseudocode about extracting all kinds of comments
```text
CommentExtrator:
If input_string's length is less than or equal to 1, return an empty CommentExtrator instance.

Initialization:
Create a state machine with an initial state of "not started".
Create an integer counter for depth, initialize it to 0.
Create a string variable to store the current comment being processed.
Create a list of Comment objects to store all extracted comments.

Loop through each character in the input string:
According to the current state of the state machine and the current character, perform the corresponding operation:

If the state is "not started":
If the current character is '/', set the state to "one slash".
If the current character is '"', set the state to "quote".

If the state is "one slash":
If the next character is also '/', add the current character to the comment content and change the state to "inline comment".
If the next character is '*', add both characters to the comment content, increment the depth counter by one, then set the state to "block comment".
Otherwise, if the depth counter is 0, set the state to "not started"; otherwise, set the state to "block comment".

If the state is "block comment":
If the current character is '*', change the state to "one star".
If the current character is '/', change the state to "one slash".
Otherwise, add the current character to the comment content.

If the state is "one star":
If the next character is '/', add the current character and the next character to the comment content and call the make_comment function.
If the next character is '*', add the current character to the comment content and set the state to "block comment".
Otherwise, add the current character to the comment content and set the state to "block comment".

If the state is "inline comment":
If the current character is '\n' or you have reached the end of the input string:
If the current character is not '\n', add it to the comment content.
Call the make_comment function.

If the state is "quote":
Handle escape quotes or backslashes:
If the current character is '' and the next character is '"', skip these two characters.
If the current character is ''' and the next character is '', skip these two characters.
If the current character is '"' and the state was 'quote', set the state to "not started".

Return a new CommentExtrator instance containing all extracted comments found.
```
The functionality of this pseudocode is to extract all comments from a provided string. It begins by checking whether the input is empty or contains only one character (in which case no extraction can occur), then initiates a state machine to track the various kinds of parsing states (such as inline or block comments).

The machine examines every individual character according to its present status and next letter read, determining actions accordingly and updating its internal state.

Once a potential new comment starting point is identified, the system records that position and begins collecting the subsequent contents into a final output.

A special variable named depth is used to aid in identifying nested block comments located within other segments of code and ensuring proper parsing occurs.

Upon completion of this process, the entire collection of found comments along with their positions and respective contents are returned.

#### Other functionality
1.`is_custom_comment(comment: &str) -> bool`: Determine whether the given string comment conforms to the format of custom comments. The format of custom comments requires starting with // followed by a non-alphanumeric or non-whitespace character.

2.`custom_opener(s: &str)` -> &str: Extract the opening part of the first line from the input string s until the first whitespace character is encountered. If the input string is empty or contains no whitespace characters, return an empty string.

3.`trim_end_unless_two_whitespaces(s: &str, is_doc_comment: bool)` -> &str: Remove trailing whitespace from the string s, unless they consist of two or more spaces.

4.`left_trim_comment_line(line: &str, style: &CommentStyle<'_>) -> (&str, bool)`: Perform left alignment on the given comment string and return whether leading whitespace was removed.

5.`find_uncommented(pat: &str) -> Option<usize>` and `find_last_uncommented(pat: &str) -> Option<usize>`: Search for uncommented substrings in the string and return their starting positions.

6.`contains_comment(text: &str) -> bool`: Determine whether the given string text contains any comments.

7.`find_comment_end(s: &str) -> Option<usize>`: Locate the position after the first comment in the string s and return its byte position.

8.`CharClasses`: An iterator to distinguish functional parts from comment parts in the code.

9.`LineClasses`: An iterator to traverse functional code and comment parts in the string, returning the character type of each line.

10.`UngroupedCommentCodeSlices`: An iterator to traverse code snippets within comments, separating them from each other.

11.`CommentCodeSlices`: An iterator to iterate over substrings of functional parts and comment parts within a string.

12.`filter_normal_code(code: &str) -> String`: Filter out comments from the given code string and return a string containing only functional code.

13.`CommentReducer`: An iterator to traverse valid characters within comments.

14.`consume_same_line_comments`: process multi comments in same line.

### TokenTree
Simplify the AST into a much simpler tree type, which we refer to as TokenTree.
```rust
function(a) {
    if (a) { 
        return 1
    } else {
        return 2
    }
}
```

`TokenTree` mainly contains two category.

* `SimpleToken` in previous code snippet,`if`,`return`,`a` are `SimpleToken`.
* `Nested` in previous code snippet, paired `()` and paired `{}` will form a `Nested` Token.

So a source program may represents like this.

```rust
pub enum TokenTree {
    SimpleToken {
        content: String,
        pos: u32,  // start position of file buffer.
    },
    Nested {
        elements: Vec<TokenTree>,
        kind: NestKind,
    },
}

pub type AST = Vec<TokenTree>;
```

Instead of dealing a lot data structure we simple the puzzle to dump `Vec<TokenTree>`. `TokenTree` is just another very simple version of `AST`.

`TokenTree` is very easy to parse,simple as.
```rust
...
if(tok == Tok::LParent){ // current token is `(`
    parse_nested(Tok::LParent);    
}
...
```

#### Handling ambiguity
Right now verything looks fine. But There are some token can be both `SimpleToken` and `Nested`. 
Typical for a language support type parameter like `Vec<X>`.

A Token `<` can be `type parameter sign` or `mathematic less than`. This can be solved by consult the `AST` before parse `TokenTree`.

Because we are writting a formatter for exist programming language. It is always easy for us to get the real `AST`. We can traval the `AST` the decide `<` is either a `SimpleToken` or `Nested`.

### Config
1.indent_size default been 2, user can set 2 or 4 by two ways:

1).command's parameter in terminal

2).vs-plugin seeting page, we'll integrate it into the aptos move analyzer later on


2.Users can enter -d on the terminal to format the move project to which the current directory belongs.
And enter -f on the terminal to format the specified single move file.


```rust
pub struct FormatConfig {
    pub indent_size: usize,
}
```

### FormatContext
1.The FormatEnv structure marks which syntax type is currently being processed.

2.The FormatContext structure holds the content of the move file being processed.
```rust
pub enum FormatEnv {
    FormatUse,
    FormatStruct,
    FormatExp,
    FormatTuple,
    FormatList,
    FormatLambda,
    FormatFun,
    FormatSpecModule,
    FormatSpecStruct,
    FormatSpecFun,
    FormatDefault,
}

pub struct FormatContext {
    pub content: String,
    pub env: FormatEnv,
}
  
impl FormatContext {
    pub fn new(content: String, env: FormatEnv) -> Self {  
        FormatContext { content, env }
    }

    pub fn set_env(&mut self, env: FormatEnv) {  
        self.env = env;  
    }  
}
```

### Format main idea
The entry point for formatting a specific file is the `Format::format_token_trees() `function. 
Within this function, we traverse the token_tree and dynamically update the `FormatContext` based on 
the current `TokenTree::Nested` information. Subsequent processing is then carried out according to 
the `FormatContext`. 

At a deeper level, different rewrite traits are applied for rewrite operations based on the 
specific `FormatEnv`. Within each `FormatContext` scenario, when processing a `TokenTree::SimpleToken`, 
we search and process any preceding comment information, performing localized comment handling. 

Whenever we encounter the end of a code block, denoted by the '}' symbol, we perform global comment processing for the entire code block.

Here are some internal interfaces of the module: 

1).`same_line_else_kw_and_brace` is used to determine whether a string is on the same line as the `else` keyword and the following curly brace.

2).`allow_single_line_let_else_block` is used to determine whether the `let` statement and the `else` statement can be on the same line.

3).`single_line_fn` determines whether a function can be displayed on a single line.

4).`rewrite_fn_base` rewrites the basic part of a function.

5).`rewrite_params` rewrites the parameter list of a function.

......


### Overall process 
`Vec<TokenTree>` is a tree type, It is very easy to decide how many ident,etc. And comment can pour into `result` base on the `pos` relate to `SimpleToken`.`pos`.

eg: format a single move file.
```rust
    let content = content.as_ref();
    let attrs: BTreeSet<String> = BTreeSet::new();
    let mut env = CompilationEnv::new(Flags::testing(), attrs);
    let filehash = FileHash::empty();
    let (defs, _) = parse_file_string(&mut env, filehash, &content)?;
    let lexer = Lexer::new(&content, filehash);
    let parse = crate::token_tree::Parser::new(lexer, &defs);
    let parse_result = parse.parse_tokens();
    let ce = CommentExtrator::new(content).unwrap();
    let mut t = FileLineMappingOneFile::default();
    t.update(&content);

    let f = Format::new(config, ce, t, parse_result, 
        FormatContext::new(content.to_string(), FormatEnv::FormatDefault));
    f.format_token_trees();
```

steps:

1).Call `parse_file_string` in move-compiler to obtain the original AST of this move file.

2).Call `parse.parse_tokens()` to obtain `Vec<TokenTree>`.

3).Call `CommentExtrator` to obtain `Vec<Comment>`.

4).Call `format_token_trees` to obtain `String` which contains formatted file content.
