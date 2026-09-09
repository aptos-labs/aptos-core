-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang.Ast

/-!
# LeanerLang comment extraction

Lean's command parser treats comments as trivia.  This scanner retains their
source text and UTF-8 ranges before syntax elaboration so ordinary Lean files
and the standalone formatter cross the same comment-preserving AST boundary.
-/

namespace LeanerLang

private partial def commentCharacterRanges (source : String) : Array (Nat × Nat) :=
  let rec blockLength (remaining : List Char) (depth consumed : Nat) : Nat :=
    match remaining with
    | [] => consumed
    | '/' :: '-' :: rest => blockLength rest (depth + 1) (consumed + 2)
    | '-' :: '/' :: rest =>
        if depth == 1 then consumed + 2
        else blockLength rest (depth - 1) (consumed + 2)
    | _ :: rest => blockLength rest depth (consumed + 1)
  -- A prime continues an identifier (`self'`), so an apostrophe opens a
  -- character literal only where an identifier cannot be running and the
  -- literal closes in the one-character or one-escape shape. Reading a prime
  -- as a quote would swallow every comment up to the next apostrophe.
  let identifierCharacter (character : Char) : Bool :=
    character.isAlphanum || character == '_' || character == '\'' ||
      character == '!' || character == '$' || character == '»'
  let characterLiteralLength (remaining : List Char) : Option Nat :=
    match remaining with
    | '\'' :: '\\' :: _ :: '\'' :: _ => some 4
    | '\'' :: character :: '\'' :: _ => if character == '\'' then none else some 3
    | _ => none
  let rec scan (remaining : List Char) (offset : Nat) (quote : Option Char)
      (escaped : Bool) (previous : Option Char) (found : Array (Nat × Nat)) :
      Array (Nat × Nat) :=
    match remaining with
    | [] => found
    | '-' :: '-' :: _ =>
        if quote.isSome then scan remaining.tail offset.succ quote false (some '-') found
        else
          let length := (remaining.takeWhile (· != '\n')).length
          scan (remaining.drop length) (offset + length) none false none
            (found.push (offset, offset + length))
    | '/' :: '-' :: rest =>
        if quote.isSome then scan rest (offset + 2) quote false (some '-') found
        else
          let length := blockLength rest 1 2
          scan (remaining.drop length) (offset + length) none false none
            (found.push (offset, offset + length))
    | character :: rest =>
        if escaped then scan rest offset.succ quote false (some character) found
        else if quote.isSome && character == '\\' then
          scan rest offset.succ quote true (some character) found
        else if quote == some character then
          scan rest offset.succ none false (some character) found
        else if quote.isNone && character == '"' then
          scan rest offset.succ (some character) false (some character) found
        else if quote.isNone && character == '\'' then
          match characterLiteralLength remaining with
          | some length =>
              if previous.any identifierCharacter then
                scan rest offset.succ none false (some character) found
              else scan (remaining.drop length) (offset + length) none false none found
          | none => scan rest offset.succ none false (some character) found
        else scan rest offset.succ quote false (some character) found
  scan source.toList 0 none false none #[]

/-- Extract every Lean line or nested block comment outside string and
character literals. Returned spans are half-open UTF-8 byte ranges. -/
def commentsOfSource (source : String) : Array Comment := Id.run do
  let mut comments := #[]
  let characters := source.toList
  for (startCharacter, endCharacter) in commentCharacterRanges source do
    let before := characters.take startCharacter
    let comment := String.ofList <| characters.drop startCharacter |>.take
      (endCharacter - startCharacter)
    let linePrefix := before.reverse.takeWhile (· != '\n')
    comments := comments.push {
      text := comment
      isDoc := comment.startsWith "--/" || comment.startsWith "/--" ||
        comment.startsWith "/-!"
      ownLine := linePrefix.all Char.isWhitespace
      span := {
        startByte := (String.ofList before).utf8ByteSize
        endByte := (String.ofList (characters.take endCharacter)).utf8ByteSize } }
  return comments

private def whitespaceBytes (source : String) (startByte endByte : Nat) : Bool :=
  let bytes := source.toUTF8.extract startByte endByte
  bytes.data.all fun byte => byte == 9 || byte == 10 || byte == 13 || byte == 32

private def documentationBody (comment : Comment) : String :=
  let text := comment.text
  if text.startsWith "--" then
    let body := text.drop 2
    let body := if body.startsWith " " then body.drop 1 else body
    body.trimAsciiEnd.toString
  else
    let body :=
      if text.startsWith "/-!" || text.startsWith "/--" then
        (text.drop 3).dropEnd 2
      else if text.startsWith "/-" then
        (text.drop 2).dropEnd 2
      else text
    body.trimAscii.toString

/-- Recover the contiguous comment group immediately preceding a command as
its declaration documentation. Any intervening source token (for example an
`import`) stops the search, so file headers are not mistaken for module docs. -/
def documentationBefore (source : String) (startByte : Nat) : String :=
  let preceding := (commentsOfSource source).filter (fun comment =>
    comment.span.endByte <= startByte) |>.reverse.toList
  let rec collect (comments : List Comment) (cursor : Nat) : List Comment :=
    match comments with
    | [] => []
    | comment :: rest =>
        if whitespaceBytes source comment.span.endByte cursor then
          comment :: collect rest comment.span.startByte
        else []
  let comments := (collect preceding startByte).reverse
  "\n".intercalate <| comments.map documentationBody

/-- Select comments owned by a parsed namespace command. Comments within the
syntax range are retained directly. A trailing comment is also retained when
only whitespace separates it from the last parsed token; Lean otherwise drops
such EOF trivia from the command's tail position. -/
def commentsForCommand (source : String) (span : Span) : Array Comment :=
  (commentsOfSource source).filter fun comment =>
    comment.span.startByte >= span.startByte &&
      (comment.span.endByte <= span.endByte ||
        (comment.span.startByte >= span.endByte &&
          whitespaceBytes source span.endByte comment.span.startByte &&
          whitespaceBytes source comment.span.endByte source.utf8ByteSize))

end LeanerLang
