"""Immutable run artifact and deterministic workspace helpers."""

from __future__ import annotations

import dataclasses
import difflib
import hashlib
import json
import os
import shutil
import stat
import time
from pathlib import Path
from typing import Any, Iterable


IGNORED_NAMES = {".git", ".claude", "build", "__pycache__"}


def canonical_json(value: Any) -> bytes:
    return json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=False).encode()


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def tree_hash(root: Path) -> str:
    """Hash relative paths, file types, executable bits, and content."""
    digest = hashlib.sha256()
    for path in _tree_files(root):
        relative = path.relative_to(root).as_posix().encode()
        mode = path.lstat().st_mode
        digest.update(len(relative).to_bytes(8, "big"))
        digest.update(relative)
        if path.is_symlink():
            kind = b"L"
            content = os.readlink(path).encode()
        else:
            kind = b"F"
            content = path.read_bytes()
        digest.update(kind)
        digest.update(b"X" if mode & stat.S_IXUSR else b"-")
        digest.update(len(content).to_bytes(8, "big"))
        digest.update(content)
    return digest.hexdigest()


def copy_snapshot(source: Path, destination: Path) -> None:
    if destination.exists():
        raise FileExistsError(f"run workspace already exists: {destination}")
    _validate_symlinks(source)
    shutil.copytree(source, destination, symlinks=True, ignore=_ignore)


def load_object(path: Path) -> dict[str, Any]:
    value = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(value, dict):
        raise ValueError(f"{path} must contain a JSON object")
    return value


def write_json(path: Path, value: Any) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    temporary = path.with_suffix(path.suffix + ".tmp")
    temporary.write_text(json.dumps(value, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    temporary.replace(path)


class JsonlWriter:
    def __init__(self, path: Path):
        path.parent.mkdir(parents=True, exist_ok=True)
        self._stream = path.open("a", encoding="utf-8")
        self._sequence = 0
        self._start = time.monotonic_ns()

    def emit(self, event: str, **fields: Any) -> None:
        self._sequence += 1
        record = {
            "schema_version": 1,
            "event": event,
            "sequence": self._sequence,
            "utc_ms": time.time_ns() // 1_000_000,
            "monotonic_us": (time.monotonic_ns() - self._start) // 1_000,
            **fields,
        }
        self._stream.write(json.dumps(record, sort_keys=True, default=_json_default) + "\n")
        self._stream.flush()

    def close(self) -> None:
        self._stream.close()

    def __enter__(self) -> "JsonlWriter":
        return self

    def __exit__(self, *_: object) -> None:
        self.close()


def workspace_diff(before: Path, after: Path) -> str:
    chunks: list[str] = []
    paths = sorted({path.relative_to(before) for path in _tree_files(before)} | {path.relative_to(after) for path in _tree_files(after)})
    for relative in paths:
        old_path = before / relative
        new_path = after / relative
        old = _text_lines(old_path)
        new = _text_lines(new_path)
        if old == new:
            continue
        chunks.extend(
            difflib.unified_diff(
                old,
                new,
                fromfile=f"a/{relative.as_posix()}",
                tofile=f"b/{relative.as_posix()}",
            )
        )
    return "".join(chunks)


def changed_paths(before: Path, after: Path) -> list[str]:
    before_map = {path.relative_to(before).as_posix(): _path_digest(path) for path in _tree_files(before)}
    after_map = {path.relative_to(after).as_posix(): _path_digest(path) for path in _tree_files(after)}
    return sorted(key for key in before_map.keys() | after_map.keys() if before_map.get(key) != after_map.get(key))


def copy_final_tree(package: Path, output: Path, attempts: int = 3) -> None:
    if output.exists():
        raise FileExistsError(f"refusing to overwrite final artifact tree: {output}")
    if attempts < 1:
        raise ValueError("copy attempts must be positive")
    for attempt in range(1, attempts + 1):
        try:
            shutil.copytree(package, output, symlinks=True, ignore=_ignore)
            return
        except (FileNotFoundError, shutil.Error):
            shutil.rmtree(output, ignore_errors=True)
            if attempt == attempts:
                raise
            # Claude Code can finish while a child tool is completing one last
            # atomic replacement. Retry from a clean destination and freeze
            # the first coherent tree for this run.
            time.sleep(0.05)


def _tree_files(root: Path) -> Iterable[Path]:
    if not root.exists():
        return []
    return (
        path
        for path in sorted(root.rglob("*"))
        if (path.is_file() or path.is_symlink())
        and not any(part in IGNORED_NAMES for part in path.relative_to(root).parts)
    )


def _path_digest(path: Path) -> str:
    if path.is_symlink():
        return "link:" + os.readlink(path)
    return sha256_file(path)


def _text_lines(path: Path) -> list[str]:
    if not path.is_file() or path.is_symlink():
        return []
    try:
        return path.read_text(encoding="utf-8").splitlines(keepends=True)
    except UnicodeDecodeError:
        return [f"<binary sha256={sha256_file(path)}>\n"]


def _validate_symlinks(root: Path) -> None:
    resolved_root = root.resolve()
    for path in root.rglob("*"):
        if path.is_symlink() and not path.resolve().is_relative_to(resolved_root):
            raise ValueError(f"snapshot symlink escapes source tree: {path}")


def _ignore(_: str, names: list[str]) -> set[str]:
    return set(names) & IGNORED_NAMES


def _json_default(value: Any) -> Any:
    if dataclasses.is_dataclass(value):
        return dataclasses.asdict(value)
    if isinstance(value, Path):
        return str(value)
    if isinstance(value, set):
        return sorted(value)
    if hasattr(value, "__dict__"):
        return {"type": type(value).__name__, **value.__dict__}
    return repr(value)
