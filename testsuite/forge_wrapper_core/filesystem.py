from __future__ import annotations

import os
import resource
import tempfile


class Filesystem:
    def write(self, filename: str, contents: bytes) -> None:
        raise NotImplementedError()

    def read(self, filename: str) -> bytes:
        raise NotImplementedError()

    def mkstemp(self) -> str:
        raise NotImplementedError()

    def rlimit(self, resource_type: int, soft: int, hard: int) -> None:
        raise NotImplementedError()

    def unlink(self, filename: str) -> None:
        raise NotImplementedError()


class LocalFilesystem(Filesystem):
    def write(self, filename: str, contents: bytes) -> None:
        with open(filename, "wb") as f:
            f.write(contents)

    def read(self, filename: str) -> bytes:
        with open(filename, "rb") as f:
            return f.read()

    def mkstemp(self) -> str:
        return tempfile.mkstemp()[1]

    def rlimit(self, resource_type: int, soft: int, hard: int) -> None:
        resource.setrlimit(resource_type, (soft, hard))

    def unlink(self, filename: str) -> None:
        os.unlink(filename)
