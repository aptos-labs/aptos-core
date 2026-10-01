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

/-- A prime continues an identifier (`self'`), so an apostrophe opens a
character literal only where an identifier cannot be running and the
literal closes in the one-character or one-escape shape. Reading a prime
as a quote would swallow every comment up to the next apostrophe. -/
private def identifierCharacter (character : Char) : Bool :=
  character.isAlphanum || character == '_' || character == '\'' ||
    character == '!' || character == '$' || character == '»'

/-- The end of the character literal opening at `pos`, when one does. -/
private def characterLiteralEnd? (source : String) (pos : String.Pos.Raw) : Option String.Pos.Raw :=
  let second := String.Pos.Raw.next source pos
  if String.Pos.Raw.atEnd source second then none
  else if String.Pos.Raw.get source second == '\\' then
    let third := String.Pos.Raw.next source second
    if String.Pos.Raw.atEnd source third then none
    else
      let fourth := String.Pos.Raw.next source third
      if !String.Pos.Raw.atEnd source fourth && String.Pos.Raw.get source fourth == '\'' then some (String.Pos.Raw.next source fourth) else none
  else if String.Pos.Raw.get source second == '\'' then none
  else
    let third := String.Pos.Raw.next source second
    if !String.Pos.Raw.atEnd source third && String.Pos.Raw.get source third == '\'' then some (String.Pos.Raw.next source third) else none

/-- The end of the line comment opening at `pos`: the newline, or the end. -/
private partial def lineCommentEnd (source : String) (pos : String.Pos.Raw) : String.Pos.Raw :=
  if String.Pos.Raw.atEnd source pos || String.Pos.Raw.get source pos == '\n' then pos else lineCommentEnd source (String.Pos.Raw.next source pos)

/-- The end of the nested block comment whose opening slash-dash ends at `pos`. -/
private partial def blockCommentEnd (source : String) (pos : String.Pos.Raw) (depth : Nat) : String.Pos.Raw :=
  if String.Pos.Raw.atEnd source pos then pos
  else
    let next := String.Pos.Raw.next source pos
    let character := String.Pos.Raw.get source pos
    if character == '/' && !String.Pos.Raw.atEnd source next && String.Pos.Raw.get source next == '-' then
      blockCommentEnd source (String.Pos.Raw.next source next) (depth + 1)
    else if character == '-' && !String.Pos.Raw.atEnd source next && String.Pos.Raw.get source next == '/' then
      if depth == 1 then String.Pos.Raw.next source next else blockCommentEnd source (String.Pos.Raw.next source next) (depth - 1)
    else blockCommentEnd source next depth

/-- Extract every Lean line or nested block comment outside string and
character literals, in one pass over the source at its byte positions.
Returned spans are half-open UTF-8 byte ranges. -/
def commentsOfSource (source : String) : Array Comment := Id.run do
  let mut comments : Array Comment := #[]
  let mut pos : String.Pos.Raw := 0
  -- Inside a string literal: the quote, and whether the last character escapes.
  let mut quote : Option Char := none
  let mut escaped := false
  -- The character before `pos`, and whether the line so far is blank.
  let mut previous : Option Char := none
  let mut lineBlank := true
  while !String.Pos.Raw.atEnd source pos do
    let character := String.Pos.Raw.get source pos
    let next := String.Pos.Raw.next source pos
    if let some opening := quote then
      if escaped then escaped := false
      else if character == '\\' then escaped := true
      else if character == opening then quote := none
      previous := some character
      lineBlank := if character == '\n' then true else lineBlank && character.isWhitespace
      pos := next
    else if character == '-' && !String.Pos.Raw.atEnd source next && String.Pos.Raw.get source next == '-' then
      let stop := lineCommentEnd source next
      let text := String.Pos.Raw.extract source pos stop
      comments := comments.push {
        text, isDoc := text.startsWith "--/", ownLine := lineBlank
        span := { startByte := pos.byteIdx, endByte := stop.byteIdx } }
      previous := none
      lineBlank := false
      pos := stop
    else if character == '/' && !String.Pos.Raw.atEnd source next && String.Pos.Raw.get source next == '-' then
      let stop := blockCommentEnd source (String.Pos.Raw.next source next) 1
      let text := String.Pos.Raw.extract source pos stop
      comments := comments.push {
        text, isDoc := text.startsWith "/--" || text.startsWith "/-!", ownLine := lineBlank
        span := { startByte := pos.byteIdx, endByte := stop.byteIdx } }
      previous := none
      lineBlank := false
      pos := stop
    else if character == '"' then
      quote := some '"'
      previous := some character
      lineBlank := false
      pos := next
    else if character == '\'' && !previous.any identifierCharacter then
      match characterLiteralEnd? source pos with
      | some stop =>
          previous := none
          lineBlank := false
          pos := stop
      | none =>
          previous := some character
          lineBlank := false
          pos := next
    else
      previous := some character
      lineBlank := if character == '\n' then true else lineBlank && character.isWhitespace
      pos := next
  return comments

/-- Whether only whitespace lies between two byte positions. -/
private partial def whitespaceBytes (source : String) (startByte endByte : Nat) : Bool :=
  go ⟨startByte⟩
where
  go (pos : String.Pos.Raw) : Bool :=
    if pos.byteIdx >= endByte || String.Pos.Raw.atEnd source pos then true
    else (String.Pos.Raw.get source pos).isWhitespace && go (String.Pos.Raw.next source pos)

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
def commentsForCommand (source : String) (span : Span) : Array Comment := Id.run do
  let comments := commentsOfSource source
  let inside := comments.filter fun comment =>
    comment.span.startByte >= span.startByte && comment.span.endByte <= span.endByte
  -- Only the first comment after the command can follow it across whitespace
  -- alone, and only as the file's last content.
  let some trailing := comments.find? (·.span.startByte >= span.endByte) | return inside
  if whitespaceBytes source span.endByte trailing.span.startByte &&
      whitespaceBytes source trailing.span.endByte source.utf8ByteSize then
    return inside.push trailing
  return inside

end LeanerLang
