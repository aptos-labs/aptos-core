/// A minimum regex implementation. Characters are assumed single-byte.
///
/// | Supported grammar | testcase-as-example   |
/// | ----------------- | --------------------- |
/// | Bracket list      | `bracket_list()`      |
/// | Meta-characters   | `meta_characters()`   |
/// | Repeat            | `repeat()`            |
/// | Capture group     | `capture_group()`     |
/// | OR operation      | `or_operator()`       |
///
/// Implementation notes.
/// The user-provided regex expression is parsed as a syntax tree (`AST`).
/// The AST converted to a Thompson NFA (https://en.wikipedia.org/wiki/Thompson%27s_construction).
/// The NFA is used directly to match the target string.
module aptos_std::regex {
    use std::option;
    use std::option::Option;
    use std::vector;

    /// A compiled regex.
    struct Regex has copy, drop, store {
        nfa: NFA,
    }

    /// A match result.
    struct Match has drop {
        haystack: vector<u8>,
        visitor: VisitorState,
    }

    /// Try compile a given regex expression.
    public fun compile(pattern: vector<u8>): Option<Regex> {
        let maybe_ast = ast_from_pattern(pattern);
        if (option::is_none(&maybe_ast)) return option::none();
        let ast = option::extract(&mut maybe_ast);
        let nfa = nfa_from_ast(ast);
        let regex = Regex {  nfa };
        option::some(regex)
    }


    /// Search a given string for a compiled regex.
    public fun match(regex: &Regex, s: vector<u8>): Option<Match> {
        let last_visit_times = vector[];
        let i = 0;
        let num_nfa_states = vector::length(&regex.nfa.states);
        while (i < num_nfa_states) {
            let visit_time = if (i == regex.nfa.start) { TIME_ZERO } else { 0 };
            vector::push_back(&mut last_visit_times, visit_time);
            i = i + 1;
        };
        let init_state = vector::borrow(&regex.nfa.states, regex.nfa.start);

        let all_zeros = vector[];
        let k = regex.nfa.num_groups;
        while (k > 0) {
            vector::push_back(&mut all_zeros, 0);
            k = k - 1;
        };

        let initial_visitor = VisitorState {
            state_idx: regex.nfa.start,
            group_start_times: all_zeros,
            group_end_times: all_zeros,
        };
        update_group_times_on_new_arrival(&mut initial_visitor, init_state, TIME_ZERO);

        let session = MatchSession {
            cur_time: TIME_ZERO,
            visitors: vector[initial_visitor],
            last_visit_times,
        };
        vector::for_each(s, |char|{
            trigger_epsilon_transitions(&regex.nfa, &mut session);
            trigger_normal_transitions(&regex.nfa, &mut session, char);
        });
        trigger_epsilon_transitions(&regex.nfa, &mut session);
        let MatchSession { cur_time: _cur_time, visitors, last_visit_times: _last_visit_times } = session;
        let (visitor_found, visitor_idx) = vector::find(&visitors, |visitor|{
            let visitor: &VisitorState = visitor;
            visitor.state_idx == regex.nfa.end
        });
        if (visitor_found) {
            let visitor = vector::swap_remove(&mut visitors, visitor_idx);
            let match = Match {
                haystack: s,
                visitor,
            };
            option::some(match)
        } else {
            option::none()
        }
    }

    /// Return `(captured, begin, end)` where,
    /// if the given group was captured at least once during the match,
    /// `captured` will be `true` and `[begin, end)` will be the position of the last capture;
    /// otherwise, `captured` will be false and `begin, end` should be ignored.
    ///
    /// See test case `capture_group()` for detailed examples.
    public fun matched_group(m: &Match, group_idx: u64): (bool, u64, u64) {
        if (group_idx == 0) {
            (true, 0, vector::length(&m.haystack))
        } else if (group_idx > vector::length(&m.visitor.group_start_times)) {
            (false, 0, 0)
        } else {
            let group_start_time = *vector::borrow(&m.visitor.group_start_times, group_idx-1);
            let group_end_time = *vector::borrow(&m.visitor.group_end_times, group_idx-1);
            if (group_end_time == 0) {
                (false, 0, 0)
            } else {
                (true, group_start_time - TIME_ZERO, group_end_time - TIME_ZERO)
            }
        }
    }

    #[test]
    fun bracket_list() {
        // matching '2' or '3' or '4'.
        let regex = option::extract(&mut compile(b"[2-4]"));
        assert!(option::is_none(&mut match(&regex, b"1")), 1);
        assert!(option::is_some(&mut match(&regex, b"2")), 2);
        assert!(option::is_some(&mut match(&regex, b"3")), 3);
        assert!(option::is_some(&mut match(&regex, b"4")), 4);
        assert!(option::is_none(&mut match(&regex, b"5")), 5);

        // matching any character but '2' and '3' and '4'.
        let regex = option::extract(&mut compile(b"[^2-4]"));
        assert!(option::is_some(&mut match(&regex, b"1")), 1);
        assert!(option::is_none(&mut match(&regex, b"2")), 2);
        assert!(option::is_none(&mut match(&regex, b"3")), 3);
        assert!(option::is_none(&mut match(&regex, b"4")), 4);
        assert!(option::is_some(&mut match(&regex, b"5")), 5);
    }

    #[test]
    fun or_operator() {
        // A regex that matches either `http`, or `https`, or `ftp`.
        let regex = option::extract(&mut compile(b"http|https|ftp"));
        assert!(option::is_some(&match(&regex, b"ftp")), 1);
        assert!(option::is_some(&match(&regex, b"https")), 2);
        assert!(option::is_some(&match(&regex, b"http")), 3);
        assert!(option::is_none(&match(&regex, b"quic")), 4);
        assert!(option::is_none(&match(&regex, b"httptp")), 5);
    }

    #[test]
    fun meta_characters() {
        // A regex that matches a single word character, equivalent to `[0-9A-Za-z_]`.
        let regex = option::extract(&mut compile(b"\\w"));
        assert!(option::is_some(&match(&regex, b"a")), 1);
        assert!(option::is_some(&match(&regex, b"z")), 2);
        assert!(option::is_some(&match(&regex, b"A")), 3);
        assert!(option::is_some(&match(&regex, b"Z")), 4);
        assert!(option::is_some(&match(&regex, b"0")), 5);
        assert!(option::is_some(&match(&regex, b"9")), 6);
        assert!(option::is_some(&match(&regex, b"_")), 7);
        assert!(option::is_none(&match(&regex, b"-")), 8);

        // A regex that matches a single digit character, equivalent to `[0-9]`.
        let regex = option::extract(&mut compile(b"\\d"));
        assert!(option::is_some(&match(&regex, b"0")), 11);
        assert!(option::is_some(&match(&regex, b"9")), 12);
        assert!(option::is_none(&match(&regex, b"=")), 13);

        // A regex that matches a single digit character, equivalent to `[ \t\r\n\f]`.
        let regex = option::extract(&mut compile(b"\\s"));
        assert!(option::is_some(&match(&regex, b" ")), 21);
        assert!(option::is_some(&match(&regex, b"\t")), 22);
        assert!(option::is_some(&match(&regex, b"\n")), 23);
        assert!(option::is_none(&match(&regex, b"_")), 24);

        // A regex that matches a single character.
        let regex = option::extract(&mut compile(b"."));
        assert!(option::is_some(&match(&regex, b" ")), 31);
        assert!(option::is_some(&match(&regex, b"\t")), 32);
        assert!(option::is_some(&match(&regex, b"\n")), 33);
        assert!(option::is_some(&match(&regex, b"_")), 34);
        assert!(option::is_some(&match(&regex, b"a")), 35);
        assert!(option::is_some(&match(&regex, b"Z")), 35);
        assert!(option::is_some(&match(&regex, b"1")), 36);
        assert!(option::is_some(&match(&regex, b"*")), 37);
        assert!(option::is_some(&match(&regex, b"?")), 38);
        assert!(option::is_some(&match(&regex, b"\0")), 39);
    }

    #[test]
    fun repeat() {
        // A regex that optionally matches `abc`.
        let regex = option::extract(&mut compile(b"(abc)?"));
        assert!(option::is_some(&match(&regex, b"abc")), 1);
        assert!(option::is_some(&match(&regex, b"")), 2);
        assert!(option::is_none(&match(&regex, b"abcab")), 3);

        // A regex that matches `abc` repeated any times.
        let regex = option::extract(&mut compile(b"(abc)*"));
        assert!(option::is_some(&match(&regex, b"")), 11);
        assert!(option::is_some(&match(&regex, b"abc")), 12);
        assert!(option::is_some(&match(&regex, b"abcabc")), 13);
        assert!(option::is_some(&match(&regex, b"abcabcabc")), 14);
        assert!(option::is_none(&match(&regex, b" abcabcabcabc")), 15);

        // A regex that matches `abc` repeated at least once.
        let regex = option::extract(&mut compile(b"(abc)+"));
        assert!(option::is_none(&match(&regex, b"")), 21);
        assert!(option::is_some(&match(&regex, b"abc")), 22);
        assert!(option::is_some(&match(&regex, b"abcabc")), 23);
        assert!(option::is_some(&match(&regex, b"abcabcabc")), 24);
        assert!(option::is_none(&match(&regex, b" abcabcabcabc")), 25);

        // A regex that matches `abc` repeated exactly twice.
        let regex = option::extract(&mut compile(b"(abc){2}"));
        assert!(option::is_none(&match(&regex, b"abc")), 31);
        assert!(option::is_some(&match(&regex, b"abcabc")), 32);
        assert!(option::is_none(&match(&regex, b"abcabcabc")), 33);

        // A regex that matches `abc` repeated 2/3/4 times.
        let regex = option::extract(&mut compile(b"(abc){2,4}"));
        assert!(option::is_none(&match(&regex, b"abc")), 41);
        assert!(option::is_some(&match(&regex, b"abcabc")), 42);
        assert!(option::is_some(&match(&regex, b"abcabcabc")), 43);
        assert!(option::is_some(&match(&regex, b"abcabcabcabc")), 44);
        assert!(option::is_none(&match(&regex, b"abcabcabcabcabc")), 45);

        // A regex that matches `abc` repeated at least twice.
        let regex = option::extract(&mut compile(b"(abc){2,}"));
        assert!(option::is_none(&match(&regex, b"abc")), 51);
        assert!(option::is_some(&match(&regex, b"abcabc")), 52);
        assert!(option::is_some(&match(&regex, b"abcabcabc")), 53);
        assert!(option::is_some(&match(&regex, b"abcabcabcabc")), 54);
        assert!(option::is_some(&match(&regex, b"abcabcabcabcabc")), 55);
    }

    #[test]
    fun capture_group() {
        // A regex that matches variable assignment, also captures the variable name and the value.
        // If the value is a list of numbers, also capture the last number.
        let regex = option::extract(&mut compile(b"([a-zA-Z0-9_]+)=(true|false|\\[(\\d+,)*\\])"));
        let haystack = b"var_0=true";
        let match = option::extract(&mut match(&regex, haystack));

        let (g1_captured, g1_start, g1_end) = matched_group(&match, 1);
        assert!(g1_captured && b"var_0" == vector::slice(&haystack, g1_start, g1_end), 901);

        let (g2_captured, g2_start, g2_end) = matched_group(&match, 2);
        assert!(g2_captured && b"true" == vector::slice(&haystack, g2_start, g2_end), 902);

        let (g3_captured, _, _) = matched_group(&match, 3);
        assert!(!g3_captured, 903);

        let haystack = b"VAR1=[22,33,55,]";
        let match = option::extract(&mut match(&regex, haystack));

        let (g1_captured, g1_start, g1_end) = matched_group(&match, 1);
        assert!(g1_captured && b"VAR1" == vector::slice(&haystack, g1_start, g1_end), 911);

        let (g2_captured, g2_start, g2_end) = matched_group(&match, 2);
        assert!(g2_captured && b"[22,33,55,]" == vector::slice(&haystack, g2_start, g2_end), 912);

        // If a group is repeated, the last capture will be returned.
        let (g3_captured, g3_start, g3_end) = matched_group(&match, 3);
        assert!(g3_captured && b"55," == vector::slice(&haystack, g3_start, g3_end), 913);

        assert!(option::is_none(&mut match(&regex, b"var.2=[22,33,55,]")), 921);
        assert!(option::is_none(&mut match(&regex, b"Var3=[22,33,55,77]")), 931);
    }

    #[test]
    fun invalid_regex() {
        // incomplete group
        assert!(option::is_none(&compile(b"(abc")), 1);

        // unexpected closing parenthesis
        assert!(option::is_none(&compile(b"(abc))")), 2);

        // invalid repeat
        assert!(option::is_none(&compile(b"(abc){2,a}")), 3);

        // invalid repeat
        assert!(option::is_none(&compile(b"(abc){2,3,44}")), 4);

        // invalid repeat
        assert!(option::is_none(&compile(b"(abc){22,3}")), 5);

        // incomplete bracket list
        assert!(option::is_none(&compile(b"[a-c")), 6);
    }

    //
    // Internal defs begin.
    //

    /// A regex expression parsed as a tree.
    ///
    /// An LL(1) grammar is used. Each line below is a production rule.
    /// REGEX ::= MULTICHARS REGEX_SUFFIX
    /// REGEX ::= ( REGEX ) GRPQUANT REGEX_SUFFIX
    /// GRPQUANT ::=
    /// GRPQUANT ::= QUANT
    /// REGEX_SUFFIX ::=
    /// REGEX_SUFFIX ::= REGEX
    /// REGEX_SUFFIX ::= | REGEX
    /// MULTICHARS ::= CHARSET MULTICHARS'
    /// MULTICHARS' ::=
    /// MULTICHARS' ::= QUANT
    /// CHARSET ::= CHAR
    /// CHARSET ::= [ CHAR MORE_CHAR ]
    /// MORE_CHAR ::=
    /// MORE_CHAR ::= CHAR MORE_CHAR
    /// CHAR ::= \ CHAR_SUFFIX
    /// CHAR can be any character except the following [ ] ( ) { } + * ? |
    /// CHAR_SUFFIX can be any character.
    /// QUANT ::= ?
    /// QUANT ::= +
    /// QUANT ::= *
    /// QUANT ::= { NUM MAYBE_RANGE_MAX }
    /// MAYBE_RANGE_MAX ::=
    /// MAYBE_RANGE_MAX ::= , RANGE_MAX
    /// RANGE_MAX ::=
    /// RANGE_MAX ::= NUM
    /// NUM ::= 0 NUM'
    /// NUM ::= 1 NUM'
    /// NUM ::= 2 NUM'
    /// NUM' ::=
    /// NUM' ::= NUM
    ///
    /// Useful tool: https://www.cs.princeton.edu/courses/archive/spring20/cos320/LL1/
    struct AST has drop {
        root: u64,
        nodes: vector<AstNode>,
        num_capture_groups: u64,
    }

    struct AstNode has copy, drop {
        idx: u64,
        type: u64,
        group_idx: u64,
        charset: u256,
        repeat_min: u64,
        repeat_max: u64,
        child_0: u64,
        child_1: u64,
    }

    /// Helps compile a bracket list in regex (e.g. `[a-z\d_-]`) to a character set it matches.
    struct BracketListParser has drop {
        /// 0: nothing like a range
        /// 1: saw a single char to start a range (stored in `range_start`), expecting a -
        /// 2: saw a range start and -, expecting a single char to finish the range
        range_state: u8,
        range_start: u8,
        accumulated_charset: u256,
        is_negated_set: bool,
        num_chars: u64,
    }

    struct NFATransition has copy, drop, store {
        activation: u256,
        to: u64,
    }

    struct NFAState has copy, drop, store {
        idx: u64,
        group_actions: vector<u64>, // group_idx + t, where t = 0 if group-begin, or 1<<63 otherwise.
        epsilon_transitions: vector<u64>,
        normal_transitions: vector<NFATransition>,
    }

    /// A [Thompson NFA](https://en.wikipedia.org/wiki/Thompson%27s_construction).
    struct NFA has copy, drop, store {
        states: vector<NFAState>,
        start: u64,
        end: u64,
        num_groups: u64,
    }

    struct MatchSession has drop {
        cur_time: u64,
        visitors: vector<VisitorState>,
        last_visit_times: vector<u64>,
    }

    struct VisitorState has copy, drop {
        state_idx: u64,
        group_start_times: vector<u64>,
        group_end_times: vector<u64>,
    }

    const AST_NODE_TYPE__CONCAT: u64 = 1;
    const AST_NODE_TYPE__OR: u64 = 2;
    const AST_NODE_TYPE__EPSILON: u64 = 3;
    const AST_NODE_TYPE__CAPTURE: u64 = 4;
    const AST_NODE_TYPE__REPEAT: u64 = 5;
    const AST_NODE_TYPE__CHARMATCH: u64 = 6;
    const NULL: u64 = 0xffffffffffffffff;
    const INF: u64 = 0xffffffffffffffff;
    const TIME_ZERO: u64 = 1 << 63;
    const GROUP_ACTION_MASK: u64 = 1 << 63;

    //
    // Internal defs end.
    // Regex -> AST begina.
    //

    fun ast_from_pattern(pattern: vector<u8>): Option<AST> {
        let ast = AST {
            root: NULL,
            nodes: vector[],
            num_capture_groups: 0,
        };
        let n = vector::length(&pattern);
        let cursor = 0;
        let num_capture_groups = 0;
        let (parsed, sub_root_idx) = parse_regex(&pattern, n, &mut cursor, &mut ast, &mut num_capture_groups);
        if (parsed && cursor == n) {
            ast.root = sub_root_idx;
            ast.num_capture_groups = num_capture_groups;
            option::some(ast)
        } else {
            option::none()
        }
    }

    fun parse_regex(tokens: &vector<u8>, end: u64, cur: &mut u64, ast: &mut AST, num_groups: &mut u64): (
        bool, // parsed?
        u64, // if parsed, the index of the representing AST node?
    )  {
        if (*cur >= end) return (false, 0);
        let token = *vector::borrow(tokens, *cur);
        let (sub_node_0_idx, sub_node_1_idx) = if (token == 40) { // (
            let cur_group_idx = *num_groups;
            *num_groups = *num_groups + 1;
            *cur = *cur + 1;
            let (sub_parsed_0, sub_node_0) = parse_regex(tokens, end, cur, ast, num_groups);
            if (!sub_parsed_0) return (false, 0);
            if (*cur >= end) return (false, 0);
            let token = *vector::borrow(tokens, *cur);
            if (token != 41) return (false, 0);   // )
            *cur = *cur + 1;
            let cur_token = if (*cur < end) { *vector::borrow(tokens, *cur) } else { 0 };
            let min = 1;
            let max = 1;
            if (cur_token == 42 || cur_token == 43 || cur_token == 63 || cur_token == 123) { // * + ? {
                let (sub_parsed, sub_min, sub_max) = parse_quantifier(tokens, end, cur);
                if (!sub_parsed) return (false, 0);
                min = sub_min;
                max = sub_max
            };

            let (sub_parsed_1, sub_node_1_idx) = parse_regex_suffix(tokens, end, cur, ast, num_groups);
            if (!sub_parsed_1) return (false, 0);
            let cap_node = AstNode {
                idx: NULL,
                type: AST_NODE_TYPE__CAPTURE,
                group_idx: cur_group_idx,
                charset: 0,
                repeat_min: NULL,
                repeat_max: NULL,
                child_0: sub_node_0,
                child_1: NULL,
            };
            let cap_node_idx = ast_add_node(ast, cap_node);
            let repeat_node_idx = ast_add_repeat_node_smart(ast, cap_node_idx, min, max);
            (repeat_node_idx, sub_node_1_idx)
        } else {
            let (parsed, charset, min, max) = parse_multichars(tokens, end, cur);
            if (!parsed) return (false, 0);
            let (parsed, sub_node_1_idx) = parse_regex_suffix(tokens, end, cur, ast, num_groups);
            if (!parsed) return (false, 0);
            let charmatch_node = AstNode {
                idx: NULL,
                type: AST_NODE_TYPE__CHARMATCH,
                group_idx: NULL,
                charset,
                child_0: NULL,
                child_1: NULL,
                repeat_max: NULL,
                repeat_min: NULL,
            };
            let charmatch_node_idx = ast_add_node(ast, charmatch_node);
            let repeat_node_idx = ast_add_repeat_node_smart(ast, charmatch_node_idx, min, max);
            (repeat_node_idx, sub_node_1_idx)
        };
        let sub_node_1 = *vector::borrow(&ast.nodes, sub_node_1_idx);
        let (ret0, ret1) = if (sub_node_1.type == AST_NODE_TYPE__OR) {
            // sub_node_0    ,       OR (sub_node_1)     ===>         OR (sub_node_1)
            //                      /  \                             /  \
            //                     /    \                           /    \
            //                    1a    1b                     CONCAT    1b
            //                                                  /  \
            //                                                 /    \
            //                                         sub_node_0   1a
            //
            let new_node_idx = ast_add_concat_node_smart(ast, sub_node_0_idx, sub_node_1.child_0);
            vector::borrow_mut(&mut ast.nodes, sub_node_1_idx).child_0 = new_node_idx;
            (true, sub_node_1_idx)
        } else {
            let new_node_idx = ast_add_concat_node_smart(ast, sub_node_0_idx, sub_node_1_idx);
            (true, new_node_idx)
        };
        (ret0, ret1)
    }

    fun parse_regex_suffix(tokens: &vector<u8>, end :u64, cur: &mut u64, ast: &mut AST, num_groups: &mut u64): (
        bool, // parsed?
        u64, // if parsed, the index of the representing AST node?
    ) {
        let cur_token = if (*cur < end) { *vector::borrow(tokens, *cur) } else { 0 };
        if (*cur >= end || cur_token == 41) { // )
            let epsilon_node = AstNode {
                idx: NULL,
                type: AST_NODE_TYPE__EPSILON,
                group_idx: NULL,
                charset: 0,
                repeat_min: NULL,
                repeat_max: NULL,
                child_0: NULL,
                child_1: NULL,
            };
            let new_node_idx = ast_add_node(ast, epsilon_node);
            (true, new_node_idx)
        } else if (cur_token == 124) { // |
            *cur = *cur + 1;
            let (sub_parsed, sub_node_idx) = parse_regex(tokens, end, cur, ast, num_groups);
            if (!sub_parsed) return (false, 0);
            let epsilon_node = AstNode {
                idx: NULL,
                type: AST_NODE_TYPE__EPSILON,
                group_idx: NULL,
                charset: 0,
                repeat_min: NULL,
                repeat_max: NULL,
                child_0: NULL,
                child_1: NULL,
            };
            let epsilon_node_idx = ast_add_node(ast, epsilon_node);

            let or_node = AstNode {
                idx: NULL,
                type: AST_NODE_TYPE__OR,
                group_idx: NULL,
                charset: 0,
                repeat_min: NULL,
                repeat_max: NULL,
                child_0: epsilon_node_idx,
                child_1: sub_node_idx,
            };
            let or_node_idx = ast_add_node(ast, or_node);
            (true, or_node_idx)
        } else {
            parse_regex(tokens, end, cur, ast, num_groups)
        }
    }

    fun parse_multichars(tokens: &vector<u8>, end: u64, cur: &mut u64): (
        bool, // parsed?
        u256, // if parsed, the charset?
        u64, // if parsed, the minimum times to repeat?
        u64, // if parsed, the maximum times to repeat?
    ) {
        let (parsed, charset) = parse_charset(tokens, end, cur);
        if (!parsed) return (false, 0, 0, 0);
        let min = 1;
        let max = 1;
        let cur_token = if (*cur < end) { *vector::borrow(tokens, *cur) } else { 0 };
        if (cur_token == 42 || cur_token == 43 || cur_token == 63 || cur_token == 123) {
            let (sub_parsed, sub_min, sub_max) = parse_quantifier(tokens, end, cur);
            if (!sub_parsed) return (false, 0, 0, 0);
            min = sub_min;
            max = sub_max;
        };
        (true, charset, min, max)
    }

    fun parse_charset(tokens: &vector<u8>, end: u64, cur: &mut u64): (
        bool, // parsed?
        u256, // if parsed, the charset?
    ) {
        if (*cur >= end) return (false, 0);
        let token = *vector::borrow(tokens, *cur);
        let charset = if (token == 91) { // [
            *cur = *cur + 1;
            let bracket_parser = new_bracket_list_parser();
            while (true) {
                if (*cur >= end) return (false, 0);
                let token = *vector::borrow(tokens, *cur);
                if (token == 93) {  // ]
                    *cur = *cur + 1;
                    break
                };
                let (parsed, char, escaped) = parse_char(tokens, end, cur);
                if (!parsed) return (false, 0);
                let bracket_parse_error = bracket_list_parser_update(&mut bracket_parser, char, escaped);
                if (bracket_parse_error) return (false, 0);
            };
            bracket_list_parser_finish(bracket_parser)
        } else {
            let (parsed, char, escaped) = parse_char(tokens, end, cur);
            if (!parsed) return (false, 0);
            let (_, charset) = get_charset(char, escaped);
            charset
        };
        (true, charset)
    }

    fun parse_quantifier(tokens: &vector<u8>, end: u64, cur: &mut u64): (
        bool, // parsed?
        u64, // if parsed, the min (inclusive)?
        u64, // if parsed, the max (inclusive)?
    ) {
        if (*cur == end) return (false, NULL, NULL);
        let token = *vector::borrow(tokens, *cur);
        *cur = *cur + 1;
        let (lo, hi) = if (token == 42) { // *
            (0, INF)
        } else if (token == 43) { // +
            (1, INF)
        } else if (token == 63) { // ?
            (0, 1)
        } else if (token == 123) { // {
            let (sub_parsed, range_min) = parse_number(tokens, end, cur);
            if (!sub_parsed) return (false, NULL, NULL);
            let (sub_parsed, has_range_max, range_max_val) = parse_maybe_range_max(tokens, end, cur);
            if (!sub_parsed) return (false, NULL, NULL);
            if (*cur >= end) return (false, NULL, NULL);
            let token = *vector::borrow(tokens, *cur); *cur = *cur + 1;
            if (token != 125) return (false, NULL, NULL); // }

            if (has_range_max) {
                if (range_max_val < range_min) return (false, NULL, NULL);
                (range_min, range_max_val)
            } else {
                (range_min, range_min)
            }
        } else {
            return (false, NULL, NULL)
        };

        (true, lo, hi)
    }

    fun parse_maybe_range_max(tokens: &vector<u8>, end: u64, cur: &mut u64): (
        bool, // parsed?
        bool, // if parsed, is there a `range_max`?
        u64, // if parsed and a `range_max` is present, its value?
    ) {
        if (*cur >= end) return (false, false, 0);
        let token = *vector::borrow(tokens, *cur);
        if (token == 125) return (true, false, 0); // }
        if (token != 44) return (false, false, 0); // ,
        *cur = *cur + 1;
        let (sub_parsed, range_max_val) = parse_range_max(tokens, end, cur);
        if (!sub_parsed) return (false, false, 0);
        (true, true, range_max_val)
    }

    fun parse_range_max(tokens: &vector<u8>, end: u64, cur: &mut u64): (
        bool, // parsed?
        u64, // if parsed, the range max value?
    ) {
        if (*cur >= end) return (false, 0);
        let token = *vector::borrow(tokens, *cur);
        if (token == 125) return (true, INF); // }
        let (sub_parsed, range_max) = parse_number(tokens, end, cur);
        if (!sub_parsed) return (false, 0);
        (true, range_max)
    }

    fun parse_number(tokens: &vector<u8>, end: u64, cur: &mut u64): (
        bool, // parsed?
        u64, // if parsed, the parsed number.
    ) {
        let acc = 0;
        while (true) {
            if (*cur >= end) break;
            let new_char = *vector::borrow(tokens, *cur);
            if (new_char < 48 || new_char > 57) break;
            acc = acc * 10 + ((new_char as u256) - 48);
            if (acc > 0xffffffffffffffff) return (false, 0);
            *cur = *cur + 1;
        };
        if (acc == 0) return (false, 0);
        (true, (acc as u64))
    }

    fun parse_char(tokens: &vector<u8>, end: u64, cur: &mut u64): (
        bool, // parsed?
        u8, // If parsed, the char value?
        bool, // If parsed, is it escaped?
    ) {
        if (*cur == end) return (false, 0, false);
        let token = *vector::borrow(tokens, *cur); *cur = *cur + 1;
        if (token == 92) { // \
            if (*cur == end) return (false, 0, false);
            let token = *vector::borrow(tokens, *cur); *cur = *cur + 1;
            (true, token, true)
        } else if (
            token == 43 // +
                || token == 42 // *
                || token == 63 // ?
                || token == 91 // [
                || token == 93 // ]
                || token == 40 // (
                || token == 41 // )
                || token == 123 // {
                || token == 124 // |
                || token == 125 // }
        ) {
            (false, 0, false)
        } else {
            (true, token, false)
        }
    }

    fun ast_add_node(ast: &mut AST, node: AstNode): u64 {
        let new_node_idx = vector::length(&ast.nodes);
        node.idx = new_node_idx;
        vector::push_back(&mut ast.nodes, node);
        new_node_idx
    }

    /// Add a CONCAT node for the given 2 child nodes.
    /// If one of the children is a EPSILON node, return the other and avoid creating a new CONCAT node.
    /// The child nodes must have been added.
    fun ast_add_concat_node_smart(ast: &mut AST, child_0_idx: u64, child_1_idx: u64): u64 {
        if (vector::borrow(&ast.nodes, child_0_idx).type == AST_NODE_TYPE__EPSILON) return child_1_idx;
        if (vector::borrow(&ast.nodes, child_1_idx).type == AST_NODE_TYPE__EPSILON) return child_0_idx;
        let new_node = AstNode {
            idx: NULL,
            type: AST_NODE_TYPE__CONCAT,
            group_idx: NULL,
            charset: 0,
            child_0: child_0_idx,
            child_1: child_1_idx,
            repeat_min: NULL,
            repeat_max: NULL,
        };
        ast_add_node(ast, new_node)
    }

    /// Add a REPEAT node for a given child node.
    /// If repeat_min == repeat_max == 1, return the child node and avoid creating a new REPEAT node.
    /// If the child is a EPSILON node, return it directly and avoid creating a new REPEAT node..
    /// The child node must have been added.
    fun ast_add_repeat_node_smart(ast: &mut AST, child_idx: u64, repeat_min: u64, repeat_max: u64): u64 {
        if (repeat_min == 1 && repeat_max == 1 || vector::borrow(&ast.nodes, child_idx).type == AST_NODE_TYPE__EPSILON) return child_idx;
        let repeat_node = AstNode {
            idx: NULL,
            type: AST_NODE_TYPE__REPEAT,
            group_idx: NULL,
            repeat_min,
            repeat_max,
            charset: 0,
            child_0: child_idx,
            child_1: NULL,
        };
        ast_add_node(ast, repeat_node)
    }

    fun new_bracket_list_parser(): BracketListParser {
        BracketListParser {
            range_start: 0,
            range_state: 0,
            accumulated_charset: 0,
            is_negated_set: false,
            num_chars: 0,
        }
    }

    /// Feed a char to a `BracketListParser`.
    /// Return whether invalid bracket list is detected.
    fun bracket_list_parser_update(parser: &mut BracketListParser, new_char_val: u8, new_char_is_escaped: bool): bool {
        parser.num_chars = parser.num_chars + 1;
        if (parser.num_chars == 1 && new_char_val == 94 && !new_char_is_escaped) { // saw a raw ^ at the beginning of the list
            parser.is_negated_set = true;
            return false
        };
        let (new_char_is_meta, new_charset) = get_charset(new_char_val, new_char_is_escaped);
        if (parser.range_state == 0) {
            if (new_char_is_meta) {
                parser.accumulated_charset = parser.accumulated_charset | new_charset;
                false
            } else {
                parser.range_state = 1;
                parser.range_start = new_char_val;
                false
            }
        } else if (parser.range_state == 1) {
            if (new_char_val == 45 && !new_char_is_escaped) { // a raw hyphen arrived!
                parser.range_state = 2;
                false
            } else if (new_char_is_meta) {
                parser.range_state = 0;
                let (_, range_start_as_charset) = get_charset(parser.range_start, false);
                parser.accumulated_charset = parser.accumulated_charset | range_start_as_charset | new_charset;
                false
            } else {
                let (_, range_start_as_charset) = get_charset(parser.range_start, false);
                parser.accumulated_charset = parser.accumulated_charset | range_start_as_charset;
                parser.range_start = new_char_val;
                false
            }
        } else if (parser.range_state == 2) {
            if (new_char_is_meta) {
                parser.range_state = 0;
                let (_, range_start_as_charset) = get_charset(parser.range_start, false);
                let (_, hyphen_as_charset) = get_charset(45, false);
                parser.accumulated_charset = parser.accumulated_charset | range_start_as_charset | hyphen_as_charset | new_charset;
                false
            } else if (new_char_val >= parser.range_start) { // valid range found!
                let equivalent_charset = 0xffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff >> (255 - new_char_val + parser.range_start) << parser.range_start;
                parser.accumulated_charset = parser.accumulated_charset | equivalent_charset;
                parser.range_state = 0;
                false
            } else { // invalid range!
                true
            }
        } else {
            abort(99999)
        }
    }

    /// Conclude a `BracketListParser` for the aggregated character set to match.
    fun bracket_list_parser_finish(parser: BracketListParser): u256 {
        let (_, range_start_as_charset) = get_charset(parser.range_start, false);
        let (_, hyphen_as_charset) = get_charset(45, false);
        let unfinished_range_as_charset = if (parser.range_state == 1) {
            range_start_as_charset
        } else if (parser.range_state == 2) {
            range_start_as_charset | hyphen_as_charset
        } else {
            0
        };
        parser.accumulated_charset = parser.accumulated_charset | unfinished_range_as_charset;
        if (parser.is_negated_set) {
            0xffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff ^ parser.accumulated_charset
        } else {
            parser.accumulated_charset
        }
    }

    //
    // Regex -> AST ends.
    // AST -> NFA begins.
    //

    fun nfa_from_ast(ast: AST): NFA {
        let nfa = NFA { states: vector[], start: NULL, end: NULL, num_groups: ast.num_capture_groups };
        let start = add_empty_state_to_nfa(&mut nfa);
        let end = add_empty_state_to_nfa(&mut nfa);
        build_nfa_from_ast_node(&ast, ast.root, &mut nfa, start, end);
        nfa.start = start;
        nfa.end = end;
        nfa
    }

    fun build_nfa_from_ast_node(ast: &AST, node_id: u64, nfa: &mut NFA, start: u64, end: u64) {
        let node = vector::borrow(&ast.nodes, node_id);
        if (node.type == AST_NODE_TYPE__CAPTURE) {
            build_nfa_from_ast_node(ast, node.child_0, nfa, start, end);
            let start_state = vector::borrow_mut(&mut nfa.states, start);
            vector::push_back(&mut start_state.group_actions, group_action_begin(node.group_idx));
            let end_state = vector::borrow_mut(&mut nfa.states, end);
            vector::push_back(&mut end_state.group_actions, group_action_end(node.group_idx));
        } else if (node.type == AST_NODE_TYPE__CONCAT) {
            let mid = add_empty_state_to_nfa(nfa);
            build_nfa_from_ast_node(ast, node.child_0, nfa, start, mid);
            build_nfa_from_ast_node(ast, node.child_1, nfa, mid, end);
        } else if (node.type == AST_NODE_TYPE__OR) {
            let sub_start_0 = add_empty_state_to_nfa(nfa);
            let sub_end_0 = add_empty_state_to_nfa(nfa);
            let sub_start_1 = add_empty_state_to_nfa(nfa);
            let sub_end_1 = add_empty_state_to_nfa(nfa);
            build_nfa_from_ast_node(ast, node.child_0, nfa, sub_start_0, sub_end_0);
            build_nfa_from_ast_node(ast, node.child_1, nfa, sub_start_1, sub_end_1);
            add_epsilon_transition(nfa, start, sub_start_0);
            add_epsilon_transition(nfa, start, sub_start_1);
            add_epsilon_transition(nfa, sub_end_0, end);
            add_epsilon_transition(nfa, sub_end_1, end);
        } else if (node.type == AST_NODE_TYPE__REPEAT) {
            let last = start;
            let i = 0;
            while (i < node.repeat_min) {
                let new_state = add_empty_state_to_nfa(nfa);
                build_nfa_from_ast_node(ast, node.child_0, nfa, last, new_state);
                last = new_state;
                i = i + 1;
            };
            if (node.repeat_max == INF) {
                let new_state = add_empty_state_to_nfa(nfa);
                add_epsilon_transition(nfa, last, new_state);
                build_nfa_from_ast_node(ast, node.child_0, nfa, new_state, end);
                add_epsilon_transition(nfa, new_state, end);
                add_epsilon_transition(nfa, end, new_state);
            } else {
                add_epsilon_transition(nfa, last, end);
                while (i < node.repeat_max) {
                    let new_state = add_empty_state_to_nfa(nfa);
                    build_nfa_from_ast_node(ast, node.child_0, nfa, last, new_state);
                    add_epsilon_transition(nfa, new_state, end);
                    last = new_state;
                    i = i + 1;
                };
            };
        } else if (node.type == AST_NODE_TYPE__CHARMATCH) {
            let start_state = vector::borrow_mut(&mut nfa.states, start);
            let transition = NFATransition {
                activation: node.charset,
                to: end,
            };
            vector::push_back(&mut start_state.normal_transitions, transition);
        } else if (node.type == AST_NODE_TYPE__EPSILON) {
            add_epsilon_transition(nfa, start, end);
        } else {
            abort(8880)
        }
    }

    fun add_empty_state_to_nfa(nfa: &mut NFA): u64 {
        let ret = vector::length(&nfa.states);
        let new_state = NFAState {
            idx: ret,
            group_actions: vector[],
            epsilon_transitions: vector[],
            normal_transitions: vector[],
        };
        vector::push_back(&mut nfa.states, new_state);
        ret
    }

    fun add_epsilon_transition(nfa: &mut NFA, from: u64, to: u64) {
        vector::push_back(&mut vector::borrow_mut(&mut nfa.states, from).epsilon_transitions, to);
    }

    //
    // AST -> NFA ends.
    // NFA execution begins.
    //

    fun trigger_epsilon_transitions(nfa: &NFA, sess: &mut MatchSession) {
        let num_active_visitors = vector::length(&sess.visitors);
        let i = 0;
        while (i < num_active_visitors) {
            let cur_visitor = *vector::borrow(&sess.visitors, i);
            let cur_state = vector::borrow(&nfa.states, cur_visitor.state_idx);
            vector::for_each_ref(&cur_state.epsilon_transitions, |next_state_idx|{
                let next_state_idx: u64 = *next_state_idx;
                let next_state_last_visit = vector::borrow_mut(&mut sess.last_visit_times, next_state_idx);
                if (*next_state_last_visit < sess.cur_time) {
                    *next_state_last_visit = sess.cur_time;
                    let new_visitor = cur_visitor;
                    new_visitor.state_idx = next_state_idx;
                    let next_state = vector::borrow(&nfa.states, next_state_idx);
                    update_group_times_on_new_arrival(&mut new_visitor, next_state, sess.cur_time);
                    vector::push_back(&mut sess.visitors, new_visitor);
                    num_active_visitors = num_active_visitors + 1;
                }
            });
            i = i + 1;
        };
    }

    fun trigger_normal_transitions(nfa: &NFA, sess: &mut MatchSession, char: u8) {
        sess.cur_time = sess.cur_time + 1;
        let new_visitors = vector[];
        vector::for_each_ref(&sess.visitors, |visitor|{
            let visitor: &VisitorState = visitor;
            let cur_state = vector::borrow(&nfa.states, visitor.state_idx);
            vector::for_each_ref(&cur_state.normal_transitions, |transition|{
                let transition: &NFATransition = transition;
                let next_state_last_visit = vector::borrow_mut(&mut sess.last_visit_times, transition.to);
                if (char_triggers_transition(char, transition) && *next_state_last_visit < sess.cur_time) {
                    *next_state_last_visit = sess.cur_time;
                    let new_visitor = *visitor;
                    new_visitor.state_idx = transition.to;
                    let next_state = vector::borrow(&nfa.states, transition.to);
                    update_group_times_on_new_arrival(&mut new_visitor, next_state, sess.cur_time);
                    vector::push_back(&mut new_visitors, new_visitor);
                }
            });
        });
        sess.visitors = new_visitors;
    }

    fun update_group_times_on_new_arrival(visitor: &mut VisitorState, new_state: &NFAState, visit_time: u64) {
        vector::for_each_ref(&new_state.group_actions, |action|{
            let (group_idx, is_end) = decode_group_action(*action);
            let record = if (is_end) {
                vector::borrow_mut(&mut visitor.group_end_times, group_idx)
            } else {
                vector::borrow_mut(&mut visitor.group_start_times, group_idx)
            };
            *record = visit_time;
        });
    }

    //
    // NFA execution ends.
    // Utils begin.
    //

    /// Given a char in regex, return whether it is considered a meta-character and the charset it matches.
    fun get_charset(char: u8, escaped: bool): (bool, u256) {
        if (escaped && char == 119) { // \w to match a word char, equivalent to [a-zA-Z0-9_]
            (true, 10633823849912963253799171395480977408)
        } else if (escaped && char == 115) { // \s to match a space char, equivalent to [ \n\r\t\f]
            (true, 4294981120)
        } else if (escaped && char == 100) { // \d to match a digit char, equivalent to [0-9]
            (true, 287948901175001088)
        } else if (!escaped && char == 46) { // . to match a char
            (true, 0xffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff)
        } else {
            (false, 1 << char)
        }
    }

    inline fun char_triggers_transition(char: u8, transition: &NFATransition): bool { (transition.activation & (1<<char)) > 0 }

    inline fun decode_group_action(group_action: u64): (u64, bool) {
        let group_idx = group_action & (GROUP_ACTION_MASK - 1);
        let is_end = (group_action & GROUP_ACTION_MASK) > 0;
        (group_idx, is_end)
    }

    /// Create a "Group begin" action for a give group.
    inline fun group_action_begin(group_idx: u64): u64 { group_idx }

    /// Create a "Group end" action for a give group.
    inline fun group_action_end(group_idx: u64): u64 { group_idx + (1 << 63) }

    // Utils end.
}
