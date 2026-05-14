/*
  Vendored from npm: highlightjs-move@0.2.0
  Browser wrapper: exposes window.hljsDefineMove for mdBook integration.
*/
(function (global) {
  'use strict';
  var module = { exports: {} };
  var exports = module.exports;
/*
Language: Move
Author: Greg Nazario <greg@gnazar.io>
Description: Move is a programming language for the Aptos blockchain, designed
             for secure and flexible smart contract development. Supports Move 2.x
             features including enums, lambdas, function values, and signed integers.
Website: https://aptos.dev
Category: smart-contracts
*/

/**
 * Highlight.js language definition for Aptos Move (v11+ API).
 *
 * Built from the Aptos Move Book (https://aptos.dev/build/smart-contracts/book)
 * and the Move Specification Language reference.
 *
 * @type {import('highlight.js').LanguageFn}
 */
module.exports = function move(hljs) {
  const regex = hljs.regex;

  // ---------------------------------------------------------------------------
  // Keywords
  // ---------------------------------------------------------------------------

  /**
   * Core language keywords from the Move Book.
   * Includes declarations, visibility modifiers, control flow, ownership,
   * abilities clause, imports, and spec-language keywords.
   */
  const KEYWORDS = [
    // Declarations
    'module',
    'script',
    'struct',
    'enum',
    'fun',
    'const',
    'use',
    'spec',
    'schema',
    // Visibility & modifiers
    'public',
    'entry',
    'native',
    'inline',
    'friend',
    'package',
    // Control flow
    'if',
    'else',
    'while',
    'loop',
    'for',
    'in',
    'match',
    'break',
    'continue',
    'return',
    'abort',
    // Variable & ownership
    'let',
    'mut',
    'move',
    'copy',
    // Abilities clause
    'has',
    // Resource annotation
    'acquires',
    // Import aliasing
    'as',
    'Self',
    // Phantom type parameter
    'phantom',
    // Enum variant test operator (Move 2.0+)
    'is',
    // Spec language keywords (treated as regular keywords)
    'pragma',
    'invariant',
    'ensures',
    'requires',
    'aborts_if',
    'aborts_with',
    'include',
    'assume',
    'assert',
    'modifies',
    'emits',
    'apply',
    'axiom',
    'forall',
    'exists',
    'choose',
    'old',
    'global',
    'with',
  ];

  /**
   * Boolean literals.
   */
  const LITERALS = ['true', 'false'];

  /**
   * Built-in primitive types and the vector generic type.
   * Unsigned integers (u8-u256), signed integers (i8-i256, Move 2.3+),
   * bool, address, signer, and vector.
   */
  const TYPES = [
    // Unsigned integers
    'u8',
    'u16',
    'u32',
    'u64',
    'u128',
    'u256',
    // Signed integers (Move 2.3+)
    'i8',
    'i16',
    'i32',
    'i64',
    'i128',
    'i256',
    // Other primitives
    'bool',
    'address',
    'signer',
    'vector',
  ];

  /**
   * Built-in functions and macros.
   * Global storage operators, the assert! macro, and freeze.
   */
  const BUILTINS = [
    // Macros
    'assert!',
    // Global storage operators
    'move_to',
    'move_from',
    'borrow_global',
    'borrow_global_mut',
    // Freeze
    'freeze',
  ];

  // ---------------------------------------------------------------------------
  // Modes
  // ---------------------------------------------------------------------------

  // Nested block comments: Move supports nesting, e.g.
  // /* outer /* inner comment */ still outer */
  const BLOCK_COMMENT = hljs.COMMENT(/\/\*/, /\*\//, { contains: ['self'] });

  /**
   * Doc comments: `///` triple-slash documentation comments.
   * Supports @-style doc tags inside.
   */
  const DOC_COMMENT = hljs.COMMENT(/\/\/\//, /$/, {
    contains: [
      {
        scope: 'doctag',
        match: /@\w+/,
      },
    ],
  });

  /**
   * Regular line comments: `// ...`
   */
  const LINE_COMMENT = hljs.COMMENT(/\/\//, /$/, {});

  /**
   * Byte string literals: `b"hello"` with backslash escape sequences.
   * These are Move's UTF-8 byte string values.
   */
  const BYTE_STRING = {
    scope: 'string',
    begin: /b"/,
    end: /"/,
    contains: [
      { match: /\\./ }, // escape sequences like \n, \\, \"
    ],
    relevance: 10,
  };

  /**
   * Hex string literals: `x"DEADBEEF"`.
   * These encode raw byte arrays from hex digits.
   */
  const HEX_STRING = {
    scope: 'string',
    begin: /x"/,
    end: /"/,
    relevance: 10,
  };

  /**
   * Number literals.
   * Supports:
   * - Hexadecimal: 0xDEAD_BEEF with optional type suffix
   * - Decimal: 1_000_000 with optional type suffix
   * - Type suffixes: u8, u16, u32, u64, u128, u256, i8, i16, i32, i64, i128, i256
   */
  const NUMBER = {
    scope: 'number',
    relevance: 0,
    variants: [
      // Hex literals with optional type suffix
      { match: /\b0x[0-9a-fA-F][0-9a-fA-F_]*(?:[ui](?:8|16|32|64|128|256))?\b/ },
      // Decimal literals with optional type suffix
      { match: /\b[0-9][0-9_]*(?:[ui](?:8|16|32|64|128|256))?\b/ },
    ],
  };

  /**
   * Address literals.
   * In Move, addresses are prefixed with `@`:
   * - Numeric: @0x1, @0xCAFE
   * - Named: @aptos_framework, @my_addr
   */
  const ADDRESS_LITERAL = {
    scope: 'symbol',
    match: /@(?:0x[0-9a-fA-F][0-9a-fA-F_]*|[a-zA-Z_]\w*)/,
    relevance: 10,
  };

  /**
   * Attributes / annotations.
   * Move uses `#[name]` and `#[name(...)]` syntax for attributes like
   * #[test], #[test_only], #[view], #[event], #[resource_group_member(...)],
   * #[expected_failure(...)], #[persistent], etc.
   */
  const ATTRIBUTE = {
    scope: 'meta',
    begin: /#\[/,
    end: /\]/,
    contains: [
      {
        // Attribute name
        scope: 'keyword',
        match: /[a-zA-Z_]\w*/,
      },
      {
        // Parenthesized arguments
        begin: /\(/,
        end: /\)/,
        contains: [
          { scope: 'string', begin: /"/, end: /"/ },
          { scope: 'number', match: /\b\d+\b/ },
          // Allow nested identifiers and :: paths inside attribute args
          { match: /[a-zA-Z_]\w*(?:::[a-zA-Z_]\w*)*/ },
          { match: /=/ },
        ],
      },
    ],
    relevance: 5,
  };

  /**
   * Module declaration.
   * `module address::name { ... }` or `module 0x1::name { ... }`
   * Highlights the module path and name.
   */
  const MODULE_DECLARATION = {
    begin: [
      /\b(?:module)\b/,
      /\s+/,
      // Module path: addr::name (possibly multiple :: segments)
      /(?:0x[0-9a-fA-F_]+|[a-zA-Z_]\w*)(?:::[a-zA-Z_]\w*)*/,
    ],
    beginScope: {
      1: 'keyword',
      3: 'title.class',
    },
    relevance: 10,
  };

  /**
   * Function declarations.
   * Matches patterns like:
   *   fun name(...)
   *   public fun name(...)
   *   public entry fun name(...)
   *   native public fun name(...)
   *   inline fun name(...)
   * Highlights the function name as title.function.
   */
  const FUNCTION_DECLARATION = {
    begin: [/\bfun\b/, /\s+/, /[a-zA-Z_]\w*/],
    beginScope: {
      1: 'keyword',
      3: 'title.function',
    },
    relevance: 10,
  };

  /**
   * Struct declarations.
   * `struct Name` or `public struct Name`
   * Highlights the struct name as title.class.
   */
  const STRUCT_DECLARATION = {
    begin: [/\bstruct\b/, /\s+/, /[A-Z]\w*/],
    beginScope: {
      1: 'keyword',
      3: 'title.class',
    },
    relevance: 10,
  };

  /**
   * Enum declarations (Move 2.0+).
   * `enum Name` with optional abilities and type parameters.
   * Highlights the enum name as title.class.
   */
  const ENUM_DECLARATION = {
    begin: [/\benum\b/, /\s+/, /[A-Z]\w*/],
    beginScope: {
      1: 'keyword',
      3: 'title.class',
    },
    relevance: 10,
  };

  /**
   * Abilities after `has` keyword.
   * Matches patterns like `has copy, drop, key, store` in struct/enum declarations.
   * Highlights the ability names as built_in.
   */
  const ABILITIES = {
    begin: /\bhas\b/,
    beginScope: 'keyword',
    end: /[{;,)]/,
    returnEnd: true,
    contains: [
      {
        scope: 'built_in',
        match: /\b(?:copy|drop|key|store)\b/,
      },
      // Allow + separator for function type abilities: `has copy + drop`
      { match: /[+,]/ },
    ],
    relevance: 5,
  };

  /**
   * Module paths with :: separator.
   * Matches qualified paths like `0x1::module_name::function_name` or
   * `aptos_framework::coin::CoinStore`.
   * Highlights the path segments as title.class.
   */
  const MODULE_PATH = {
    scope: 'title.class',
    match: /\b(?:0x[0-9a-fA-F_]+|[a-zA-Z_]\w*)(?:::[a-zA-Z_]\w*)+/,
    relevance: 0,
  };

  /**
   * Function invocations.
   * Matches `identifier(` patterns but excludes keywords that look like
   * function calls (if, while, match, etc.).
   */
  const FUNCTION_INVOKE = {
    scope: 'title.function.invoke',
    relevance: 0,
    begin: regex.concat(
      /\b/,
      /(?!let\b|for\b|while\b|if\b|else\b|match\b|loop\b|return\b|abort\b|break\b|continue\b|use\b|module\b|struct\b|enum\b|fun\b|spec\b|const\b)/,
      hljs.IDENT_RE,
      regex.lookahead(/\s*(?:<[^>]*>)?\s*\(/),
    ),
  };

  /**
   * `self` as a receiver parameter / variable reference.
   * In Move, `self` is used as the receiver in method-style function declarations
   * (e.g., `fun is_eq(self: &Ordering): bool`) and in expressions (`self.field`).
   */
  const SELF_VARIABLE = {
    scope: 'variable.language',
    match: /\bself\b/,
    relevance: 0,
  };

  /**
   * vector literal constructor syntax.
   * `vector[1, 2, 3]` or `vector<u8>[1, 2, 3]`.
   * Highlights `vector` as a built-in keyword.
   */
  const VECTOR_LITERAL = {
    match: /\bvector\s*(?:<[^>]*>)?\s*\[/,
    scope: 'built_in',
    returnEnd: true,
    relevance: 5,
  };

  /**
   * Lambda / closure parameters within pipe delimiters.
   * Matches `|param1, param2|` and `||` (empty closures) in lambda expressions
   * and function type annotations.
   * Highlights the pipes as punctuation and parameters as params.
   */
  const LAMBDA_PARAMS = {
    begin: /\|/,
    end: /\|/,
    scope: 'params',
    relevance: 0,
    contains: [
      {
        scope: 'type',
        match:
          /\b(?:u8|u16|u32|u64|u128|u256|i8|i16|i32|i64|i128|i256|bool|address|signer|vector)\b/,
      },
      { match: /&\s*mut\b/, scope: 'keyword' },
      { match: /&/, scope: 'keyword' },
      NUMBER,
    ],
  };

  // ---------------------------------------------------------------------------
  // Language definition
  // ---------------------------------------------------------------------------

  return {
    name: 'Move',
    aliases: ['move', 'aptos-move', 'move-on-aptos', 'move-lang'],
    unicodeRegex: true,
    keywords: {
      $pattern: `${hljs.IDENT_RE}!?`,
      keyword: KEYWORDS,
      literal: LITERALS,
      type: TYPES,
      built_in: BUILTINS,
    },
    contains: [
      // Comments (doc comments must come before line comments to match first)
      DOC_COMMENT,
      LINE_COMMENT,
      BLOCK_COMMENT,

      // Strings
      BYTE_STRING,
      HEX_STRING,

      // Numbers
      NUMBER,

      // Address literals (@0x1, @named)
      ADDRESS_LITERAL,

      // Attributes (#[test], #[resource_group_member(...)])
      ATTRIBUTE,

      // Declarations (order matters: more specific patterns first)
      MODULE_DECLARATION,
      FUNCTION_DECLARATION,
      STRUCT_DECLARATION,
      ENUM_DECLARATION,

      // Abilities after `has`
      ABILITIES,

      // Module-qualified paths (0x1::module::item)
      MODULE_PATH,

      // vector[...] constructor
      VECTOR_LITERAL,

      // Lambda / closure params
      LAMBDA_PARAMS,

      // self as variable.language
      SELF_VARIABLE,

      // Function invocations
      FUNCTION_INVOKE,
    ],
  };
};
  global.hljsDefineMove = module.exports;
})(window);
