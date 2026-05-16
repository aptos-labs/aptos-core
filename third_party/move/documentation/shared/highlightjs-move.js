/*
  Vendored from npm: highlightjs-move@0.2.0 (v10 grammar — for highlight.js v10
  bundled by mdBook). Browser wrapper: exposes window.hljsDefineMove.
*/
(function (global) {
  "use strict";
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
 * Highlight.js v10 compatible language definition for Aptos Move.
 *
 * This file uses the v10 grammar API (className instead of scope, begin instead
 * of match, no beginScope/endScope, no hljs.regex). For highlight.js v11+,
 * use the main entry point instead.
 *
 * Built from the Aptos Move Book (https://aptos.dev/build/smart-contracts/book)
 * and the Move Specification Language reference.
 */
module.exports = function move(hljs) {
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
  // Modes (v10 compatible: uses className instead of scope, begin instead of match)
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
        className: 'doctag',
        begin: /@\w+/,
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
    className: 'string',
    begin: /b"/,
    end: /"/,
    contains: [
      { begin: /\\./ }, // escape sequences like \n, \\, \"
    ],
    relevance: 10,
  };

  /**
   * Hex string literals: `x"DEADBEEF"`.
   * These encode raw byte arrays from hex digits.
   */
  const HEX_STRING = {
    className: 'string',
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
    className: 'number',
    relevance: 0,
    variants: [
      // Hex literals with optional type suffix
      { begin: /\b0x[0-9a-fA-F][0-9a-fA-F_]*(?:[ui](?:8|16|32|64|128|256))?\b/ },
      // Decimal literals with optional type suffix
      { begin: /\b[0-9][0-9_]*(?:[ui](?:8|16|32|64|128|256))?\b/ },
    ],
  };

  /**
   * Address literals.
   * In Move, addresses are prefixed with `@`:
   * - Numeric: @0x1, @0xCAFE
   * - Named: @aptos_framework, @my_addr
   */
  const ADDRESS_LITERAL = {
    className: 'symbol',
    begin: /@(?:0x[0-9a-fA-F][0-9a-fA-F_]*|[a-zA-Z_]\w*)/,
    relevance: 10,
  };

  /**
   * Attributes / annotations.
   * Move uses `#[name]` and `#[name(...)]` syntax for attributes like
   * #[test], #[test_only], #[view], #[event], #[resource_group_member(...)],
   * #[expected_failure(...)], #[persistent], etc.
   */
  const ATTRIBUTE = {
    className: 'meta',
    begin: /#\[/,
    end: /\]/,
    contains: [
      {
        // Attribute name
        className: 'keyword',
        begin: /[a-zA-Z_]\w*/,
      },
      {
        // Parenthesized arguments
        begin: /\(/,
        end: /\)/,
        contains: [
          { className: 'string', begin: /"/, end: /"/ },
          { className: 'number', begin: /\b\d+\b/ },
          // Allow nested identifiers and :: paths inside attribute args
          { begin: /[a-zA-Z_]\w*(?:::[a-zA-Z_]\w*)*/ },
          { begin: /=/ },
        ],
      },
    ],
    relevance: 5,
  };

  /**
   * Module declaration.
   * `module address::name { ... }` or `module 0x1::name { ... }`
   * In v10, we use beginKeywords to match `module` and a sub-mode for the path.
   */
  const MODULE_DECLARATION = {
    beginKeywords: 'module',
    end: /[{;]/,
    returnEnd: true,
    contains: [
      {
        className: 'title',
        begin: /(?:0x[0-9a-fA-F_]+|[a-zA-Z_]\w*)(?:::[a-zA-Z_]\w*)*/,
        relevance: 0,
      },
    ],
    relevance: 10,
  };

  /**
   * Function declarations.
   * Matches `fun name` and highlights the function name.
   * In v10, we use beginKeywords for `fun` and a sub-mode for the name.
   */
  const FUNCTION_DECLARATION = {
    beginKeywords: 'fun',
    end: /[({;]/,
    returnEnd: true,
    contains: [
      {
        className: 'title',
        begin: /[a-zA-Z_]\w*/,
        relevance: 0,
      },
    ],
    relevance: 10,
  };

  /**
   * Struct declarations.
   * `struct Name` or `public struct Name`
   * Highlights the struct name as title.
   */
  const STRUCT_DECLARATION = {
    beginKeywords: 'struct',
    end: /[{(;]|\bhas\b/,
    returnEnd: true,
    contains: [
      {
        className: 'title',
        begin: /[A-Z]\w*/,
        relevance: 0,
      },
    ],
    relevance: 10,
  };

  /**
   * Enum declarations (Move 2.0+).
   * `enum Name` with optional abilities and type parameters.
   * Highlights the enum name as title.
   */
  const ENUM_DECLARATION = {
    beginKeywords: 'enum',
    end: /[{]|\bhas\b/,
    returnEnd: true,
    contains: [
      {
        className: 'title',
        begin: /[A-Z]\w*/,
        relevance: 0,
      },
    ],
    relevance: 10,
  };

  /**
   * Abilities after `has` keyword.
   * Matches patterns like `has copy, drop, key, store` in struct/enum declarations.
   * Highlights the ability names as built_in.
   */
  const ABILITIES = {
    begin: /\bhas\b/,
    end: /[{;,)]/,
    returnEnd: true,
    keywords: 'has',
    contains: [
      {
        className: 'built_in',
        begin: /\b(?:copy|drop|key|store)\b/,
      },
      // Allow + separator for function type abilities: `has copy + drop`
      { begin: /[+,]/ },
    ],
    relevance: 5,
  };

  /**
   * Module paths with :: separator.
   * Matches qualified paths like `0x1::module_name::function_name` or
   * `aptos_framework::coin::CoinStore`.
   * Highlights the path segments as title (maps to hljs-title).
   */
  const MODULE_PATH = {
    className: 'title',
    begin: /\b(?:0x[0-9a-fA-F_]+|[a-zA-Z_]\w*)(?:::[a-zA-Z_]\w*)+/,
    relevance: 0,
  };

  /**
   * Function invocations.
   * Matches `identifier(` patterns but excludes keywords that look like
   * function calls (if, while, match, etc.).
   * In v10, we build the regex manually without hljs.regex.
   */
  const FUNCTION_INVOKE = {
    className: 'title function_',
    relevance: 0,
    begin:
      /\b(?!let\b|for\b|while\b|if\b|else\b|match\b|loop\b|return\b|abort\b|break\b|continue\b|use\b|module\b|struct\b|enum\b|fun\b|spec\b|const\b)[a-zA-Z_]\w*(?=\s*(?:<[^>]*>)?\s*\()/,
  };

  /**
   * `self` as a receiver parameter / variable reference.
   * In Move, `self` is used as the receiver in method-style function declarations
   * (e.g., `fun is_eq(self: &Ordering): bool`) and in expressions (`self.field`).
   */
  const SELF_VARIABLE = {
    className: 'variable language_',
    begin: /\bself\b/,
    relevance: 0,
  };

  /**
   * vector literal constructor syntax.
   * `vector[1, 2, 3]` or `vector<u8>[1, 2, 3]`.
   * Highlights `vector` as a built-in keyword.
   */
  const VECTOR_LITERAL = {
    begin: /\bvector\s*(?:<[^>]*>)?\s*\[/,
    className: 'built_in',
    returnEnd: true,
    relevance: 5,
  };

  /**
   * Lambda / closure parameters within pipe delimiters.
   * Matches `|param1, param2|` and `||` (empty closures) in lambda expressions
   * and function type annotations.
   */
  const LAMBDA_PARAMS = {
    begin: /\|/,
    end: /\|/,
    className: 'params',
    relevance: 0,
    contains: [
      {
        className: 'type',
        begin:
          /\b(?:u8|u16|u32|u64|u128|u256|i8|i16|i32|i64|i128|i256|bool|address|signer|vector)\b/,
      },
      { begin: /&\s*mut\b/, className: 'keyword' },
      { begin: /&/, className: 'keyword' },
      NUMBER,
    ],
  };

  // ---------------------------------------------------------------------------
  // Language definition
  // ---------------------------------------------------------------------------

  return {
    name: 'Move',
    aliases: ['move', 'aptos-move', 'move-on-aptos', 'move-lang'],
    keywords: {
      $pattern: `${hljs.IDENT_RE}!?`,
      keyword: KEYWORDS.join(' '),
      literal: LITERALS.join(' '),
      type: TYPES.join(' '),
      built_in: BUILTINS.join(' '),
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
