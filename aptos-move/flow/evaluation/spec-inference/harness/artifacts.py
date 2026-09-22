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
    validate_symlinks(source)
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
            _defuse_escaping_symlinks(output)
            return
        except (FileNotFoundError, shutil.Error):
            shutil.rmtree(output, ignore_errors=True)
            if attempt == attempts:
                raise
            # Claude Code can finish while a child tool is completing one last
            # atomic replacement. Retry from a clean destination and freeze
            # the first coherent tree for this run.
            time.sleep(0.05)


def _defuse_escaping_symlinks(root: Path) -> list[str]:
    """Replace symlinks in a finished run's tree that point outside it.

    `final/` is the record of what an evaluated agent produced, and the agent
    can create symlinks in its own workspace. Copying one verbatim would leave
    an absolute link into the host in an artifact that is later read, exported
    and diffed -- a payload waiting for whatever follows it.

    Refusing to finalize would be worse than the disease: it hands the agent a
    way to destroy its own run's record. So the link is turned into an inert
    file naming what it pointed at, which keeps the evidence and removes the
    hazard. Snapshot ingestion still refuses outright, because there the tree
    is supposed to be trusted input rather than untrusted output.
    """
    resolved_root = root.resolve()
    defused: list[str] = []
    for path in sorted(_walk(root)):
        if not path.is_symlink():
            continue
        target = path.readlink()
        try:
            destination = (path.parent / target).resolve()
        except OSError:
            # A circular or otherwise unresolvable link. Letting this raise
            # would abort finalize before `judge.json` is written, which is the
            # very outcome this function exists to deny the agent: one planted
            # link discarding a finished session. It cannot be shown to stay
            # inside the tree, so it is defused like any other.
            destination = None
        if destination is not None and (
            destination == resolved_root or destination.is_relative_to(resolved_root)
        ):
            continue
        relative = path.relative_to(root).as_posix()
        path.unlink()
        reason = (
            "which is outside the artifact tree"
            if destination is not None
            else "which could not be resolved"
        )
        path.write_text(
            f"[symlink removed from the run record: pointed at {target}, {reason}]\n",
            encoding="utf-8",
        )
        defused.append(relative)
    return defused


def _walk(root: Path) -> Iterable[Path]:
    """Every entry beneath `root`, never descending through a symlink.

    `Path.rglob` stopped following directory links in Python 3.13, but this
    walk decides what gets hashed, rewritten and unlinked, so it states the
    property itself rather than inheriting it from the interpreter. A link to
    `..` or `/` in the agent's workspace would otherwise take a walk outside
    the tree it was given -- and one of these callers writes.
    """
    if not root.exists():
        return
    for parent, directories, files in os.walk(root, followlinks=False):
        base = Path(parent)
        # A symlinked directory is an entry in its own right, and not a way in.
        links = [name for name in directories if (base / name).is_symlink()]
        directories[:] = sorted(name for name in directories if name not in links)
        for name in sorted(files) + sorted(links):
            yield base / name


def _tree_files(root: Path) -> Iterable[Path]:
    return (
        path
        for path in sorted(_walk(root))
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


def validate_symlinks(root: Path) -> None:
    resolved_root = root.resolve()
    for path in _walk(root):
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
