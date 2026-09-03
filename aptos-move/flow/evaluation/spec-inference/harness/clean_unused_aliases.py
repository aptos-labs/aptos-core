"""Remove only compiler-identified unused `use` aliases from a corpus package."""

from __future__ import annotations

import argparse
import json
import re
from collections import defaultdict
from pathlib import Path
from typing import Any

from .artifacts import write_json
from .prepare import _atomic_text


_ALIAS = re.compile(r"Unused 'use' of alias '([^']+)'")
_USE_LINE = re.compile(r"^\s*use\s+.+;\s*$")


def clean_unused_aliases(
    package: Path, workspace_root: Path, compile_report: Path, output: Path
) -> dict[str, Any]:
    report = json.loads(compile_report.read_text(encoding="utf-8"))
    if "records" not in report:
        raise ValueError(
            f"{compile_report} carries no diagnostic records; it was written by a "
            f"flow older than compile-report schema 2"
        )
    # Each warning is anchored at the span of the one alias it is about, so a
    # `use` line that binds several aliases can be reported more than once and
    # only the reported ones may be removed.
    reported: dict[Path, dict[int, list[tuple[int, str]]]] = defaultdict(
        lambda: defaultdict(list)
    )
    for record in report["records"]:
        if record["headline"] != "unused alias":
            continue
        if record["file"] is None or record["line"] is None or record["column"] is None:
            raise ValueError(f"unused-alias warning has no source location: {record}")
        alias = _ALIAS.search(record.get("label") or "")
        if alias is None:
            raise ValueError(f"unused-alias warning names no alias: {record}")
        source = Path(record["file"])
        path = source if source.is_absolute() else workspace_root / source
        path = path.resolve()
        try:
            path.relative_to(package.resolve())
        except ValueError as error:
            raise ValueError(f"warning path is outside package: {path}") from error
        reported[path][record["line"]].append((record["column"], alias.group(1)))

    records = []
    for path, by_line in sorted(reported.items(), key=lambda item: str(item[0])):
        lines = path.read_text(encoding="utf-8").splitlines(keepends=True)
        for line_number in sorted(by_line, reverse=True):
            if not 1 <= line_number <= len(lines):
                raise ValueError(f"warning line is outside {path}: {line_number}")
            line = lines[line_number - 1]
            if not _USE_LINE.fullmatch(line.rstrip("\r\n")):
                raise ValueError(
                    f"refusing to remove non-use line {path}:{line_number}: {line.rstrip()}"
                )
            rewritten = _remove_aliases(line, by_line[line_number])
            action = "removed_use_line" if rewritten is None else "removed_alias"
            for _, alias in sorted(by_line[line_number]):
                records.append(
                    {
                        "path": path.relative_to(package).as_posix(),
                        "line": line_number,
                        "alias": alias,
                        "source": line.strip(),
                        "action": action,
                    }
                )
            if rewritten is None:
                lines.pop(line_number - 1)
            else:
                lines[line_number - 1] = rewritten
        _atomic_text(path, "".join(lines))

    result = {
        "schema_version": 2,
        "package": str(package),
        "compile_report": str(compile_report),
        "removed_count": len(records),
        "removed": sorted(
            records, key=lambda record: (record["path"], record["line"], record["alias"])
        ),
    }
    write_json(output, result)
    return result


def _remove_aliases(line: str, reported: list[tuple[int, str]]) -> str | None:
    """Drop the reported aliases from one `use` line.

    Returns the rewritten line, or `None` when the whole line goes.

    A `use` line may bind several aliases -- `use a::m::{Self, T};` binds both
    `m` and `T` -- and only some of them need be unused, so removing the line
    on the first warning would delete bindings that are still in use. Each
    reported alias is removed on its own, and the line goes only once nothing
    it binds remains.
    """
    body = line.rstrip("\r\n")
    ending = line[len(body) :]
    opening = body.find("{")
    closing = body.rfind("}")
    if opening < 0 or closing < opening:
        # No member group: the declaration binds exactly one alias.
        return None
    members = _group_members(body, opening, closing)

    dropped = set()
    for column, _ in reported:
        offset = column - 1
        member = next(
            (index for index, (start, end) in enumerate(members) if start <= offset < end),
            None,
        )
        if member is not None:
            dropped.add(member)
            continue
        # The module alias is anchored at the module name in the path rather
        # than at the `Self` that introduces it, so it falls outside the group.
        member = next(
            (
                index
                for index, (start, end) in enumerate(members)
                if body[start:end].strip() == "Self"
            ),
            None,
        )
        if member is None:
            raise ValueError(f"unused alias at column {column} is not in {body.strip()}")
        dropped.add(member)

    keep = [
        body[start:end].strip()
        for index, (start, end) in enumerate(members)
        if index not in dropped
    ]
    if not keep:
        return None
    return body[: opening + 1] + ", ".join(keep) + body[closing:] + ending


def _group_members(body: str, opening: int, closing: int) -> list[tuple[int, int]]:
    """Half-open offset spans of the comma-separated members between the braces."""
    members = []
    start = opening + 1
    depth = 0
    for index in range(opening + 1, closing):
        character = body[index]
        if character == "{":
            depth += 1
        elif character == "}":
            depth -= 1
        elif character == "," and depth == 0:
            members.append((start, index))
            start = index + 1
    members.append((start, closing))
    return [span for span in members if body[span[0] : span[1]].strip()]


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--package", type=Path, required=True)
    parser.add_argument("--workspace-root", type=Path, required=True)
    parser.add_argument("--compile-report", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    result = clean_unused_aliases(
        args.package.resolve(),
        args.workspace_root.resolve(),
        args.compile_report.resolve(),
        args.output.resolve(),
    )
    print(json.dumps({"removed_count": result["removed_count"]}, sort_keys=True))


if __name__ == "__main__":
    main()
