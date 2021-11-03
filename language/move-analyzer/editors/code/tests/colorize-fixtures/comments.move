// Line comment.

/// Documentation line comment.
/// Nested block commment: /*

/*
 Block comment.
 // Nested line comment. */

/**
 * Documentation block comment.
 * ...
 /// Nested line comment. */

/*
 * Nested
 * /* block
 *  * comments
 *  * /* create
 *  *  * a
 *  *  */ comment
 *  */ stack.
 */

/* Asterisks: *, ٭, ⁎, ∗, ⚹, ✱ */
// Slashes: /, ǀ,  ̸, ⁄, ∕, ⹊

/*
 * Whitespace:
 * >< \u000b line tabulation
 * >< \u000c form feed
 * >
< \u000d carriage return
 * >< \u0085 next line
 * > < \u00a0 no-break space
 * > < \u1680 ogham space mark
 * >᠎< \u180e mongolian vowel separator
 * > < \u2000 en quad
 * > < \u2001 em quad
 * > < \u2002 en space
 * > < \u2003 em space
 * > < \u2004 three-per-em space
 * > < \u2005 four-per-em space
 * > < \u2006 six-per-em space
 * > < \u2007 figure space
 * > < \u2008 punctuation space
 * > < \u2009 thin space
 * > < \u200a hair space
 * >​< \u200b zero width space
 * >‌< \u200c zero width non-joiner
 * >‍< \u200d zero width joiner
 * > < \u2028 line separator
 * > < \u2029 paragraph separator
 * > < \u202f narrow no-break space
 * > < \u205f medium mathematical space
 * >⁠< \u2060 word joiner
 * >　< \u3000 ideographic space
 * >﻿< \ufeff zero width non-breaking space
 */

// All block comments above are closed, so this ought to be treated as
// uncommented source code:
address 0x1 {}

// `/**/` can be tricky: it's a block comment that is opened `/*` and
// immediately closed `*/`, but a bad regular expression could treat it as a
// documentation block comment marker `/**` followed by a lone `/`:
/**/ address 0x2 {}
/***/ address 0x3 {}

// Test that the "Trojan source" vulnerability is mitigated by the TextMate language grammar.
// See the byte representation in https://trojansource.codes/trojan-source.pdf, figure 3, where
// U+202E is RLO, U+2066 is LRI, U+2069 is PDI:
//
// ```
// /*<U+202E> } <U+2066>if (isAdmin)<U+2069> <U+2066> begin admins only */
//     printf("You are an admin.\n");
// /* end admin only <U+202E> { <U+2066>*/
// ```
//
// `if (isAdmin) {` and `}` should be tokenized as comments.
fun trojan_source() {
  let isAdmin = false;
  /*‮ } ⁦if (isAdmin)⁩ ⁦ begin admins only */
      performPrivilegedOperation();
  /* end admin only ‮ { ⁦*/
}

// FIXME: In VS Code, the comment extends until the carriage return `\r`, then
// ends. Instead, line comments in Move extend until a line feed `\n`, and so
// everything up to and including "return" should be part of the line comment.
// This is important because VS Code renders the text past the "<" below as if
// if were NOT commented out, when in fact it is:
// >
< \u000d carriage return
