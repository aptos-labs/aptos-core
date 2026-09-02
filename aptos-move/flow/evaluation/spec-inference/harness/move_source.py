"""Lexical helpers for reading Move source without a compiler."""

from __future__ import annotations

import re
from dataclasses import dataclass


def mask_comments_and_strings(text: str) -> str:
    """Blank out comments and string bodies, preserving offsets and newlines.

    Pattern scans over Move source ask about code, not about the text that
    merely mentions it. Masking rather than deleting keeps every byte offset
    and line number valid, so a match in the result indexes the original.
    """
    chars = list(text)
    index = 0
    state = "code"
    while index < len(chars):
        pair = text[index : index + 2]
        if state == "code" and pair == "//":
            state = "line-comment"
            chars[index] = chars[index + 1] = " "
            index += 2
            continue
        if state == "code" and pair == "/*":
            state = "block-comment"
            chars[index] = chars[index + 1] = " "
            index += 2
            continue
        if state == "block-comment" and pair == "*/":
            chars[index] = chars[index + 1] = " "
            state = "code"
            index += 2
            continue
        char = text[index]
        if state == "code" and char == '"':
            state = "string"
            chars[index] = " "
        elif state == "string" and char == '"' and (
            index == 0 or text[index - 1] != "\\"
        ):
            chars[index] = " "
            state = "code"
        elif state != "code" and char not in "\r\n":
            chars[index] = " "
        elif state == "line-comment" and char in "\r\n":
            state = "code"
        index += 1
    return "".join(chars)


@dataclass(frozen=True)
class SpecBlock:
    """One textual `spec <function> { ... }` block, as offsets into the source.

    Offsets index the unmasked source too: masking preserves every position.
    """

    start: int
    """Offset of the `spec` keyword."""
    brace: int
    """Offset of the opening brace."""
    end: int
    """Offset just past the closing brace."""
    indent: str
    """Leading whitespace of the line the block opens on."""


def closing_brace(masked: str, opening: int) -> int | None:
    """Offset just past the brace matching the one at `opening`, if it closes."""
    depth = 0
    for index in range(opening, len(masked)):
        if masked[index] == "{":
            depth += 1
        elif masked[index] == "}":
            depth -= 1
            if depth == 0:
                return index + 1
    return None


def function_spec_blocks(masked: str, function: str) -> list[SpecBlock]:
    """Locate every `spec <function> { ... }` block in masked Move source.

    Takes text already run through `mask_comments_and_strings` so a caller that
    also needs the masked form pays for one pass. A `spec <function>;` schema
    reference names the function without opening a block and is skipped, as is
    `spec <function>: ...`, which declares a schema of that name instead.
    """
    pattern = re.compile(
        rf"(?m)^(?P<indent>[ \t]*)spec[ \t]+{re.escape(function)}\b(?![ \t]*:)"
    )
    blocks = []
    for match in pattern.finditer(masked):
        brace = masked.find("{", match.end())
        semicolon = masked.find(";", match.end())
        if brace < 0 or (semicolon >= 0 and semicolon < brace):
            continue
        end = closing_brace(masked, brace)
        if end is None:
            raise ValueError(f"unterminated spec block for {function}")
        blocks.append(
            SpecBlock(
                start=match.start(),
                brace=brace,
                end=end,
                indent=match.group("indent"),
            )
        )
    return blocks
