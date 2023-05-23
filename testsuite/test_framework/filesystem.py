# A wrapper around filesystem operations

from __future__ import annotations

import os
import resource
import tempfile
from typing import Dict, List, Optional


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

    def exists(self, filename: str) -> bool:
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

    def exists(self, filename: str) -> bool:
        return os.path.exists(filename)


class FakeFilesystem(Filesystem):
    def write(self, filename: str, contents: bytes) -> None:
        print(f"Wrote {contents} to {filename}")

    def read(self, filename: str) -> bytes:
        return b"fake"

    def mkstemp(self) -> str:
        return "temp"

    def rlimit(self, resource_type: int, soft: int, hard: int) -> None:
        return

    def unlink(self, filename: str) -> None:
        return


# Special bytestring for file not found
FILE_NOT_FOUND = b"FILE_NOT_FOUND"


class SpyFilesystem(FakeFilesystem):
    def __init__(
        self,
        expected_writes: Dict[str, bytes],
        expected_reads: Dict[str, bytes],
        expected_unlinks: Optional[List[str]] = None,
    ) -> None:
        self.expected_writes = expected_writes
        self.expected_reads = expected_reads
        self.expected_unlinks = expected_unlinks or []
        self.writes = {}
        self.reads = []
        self.temp_count = 1
        self.unlinks = []

    def write(self, filename: str, contents: bytes) -> None:
        self.writes[filename] = contents

    def get_write(self, filename: str) -> bytes:
        return self.writes[filename]

    def read(self, filename: str) -> bytes:
        self.reads.append(filename)
        return self.expected_reads.get(filename, b"")

    def assert_writes(self, testcase) -> None:
        for filename, contents in self.expected_writes.items():
            testcase.assertIn(
                filename, self.writes, f"{filename} was not written: {self.writes}"
            )
            testcase.assertMultiLineEqual(
                self.writes[filename].decode(),
                contents.decode(),
                f"{filename} did not match expected contents",
            )

    def assert_reads(self, testcase) -> None:
        for filename, content in self.expected_reads.items():
            # if content == FILE_NOT_FOUND:
            #     testcase.assertNotIn(
            #         filename, self.reads, f"{filename} was not expected to be read"
            #     )
            #     continue
            testcase.assertIn(filename, self.reads, f"{filename} was not read")

        # Check all reads and ensure that they were expected
        for filename in self.reads:
            testcase.assertIn(
                filename, self.expected_reads, f"{filename} was not expected to be read"
            )

    def mkstemp(self) -> str:
        filename = f"temp{self.temp_count}"
        self.temp_count += 1
        return filename

    def unlink(self, filename: str) -> None:
        self.unlinks.append(filename)

    def assert_unlinks(self, testcase) -> None:
        for filename in self.expected_unlinks:
            testcase.assertIn(filename, self.unlinks, f"{filename} was not unlinked")

    def exists(self, filename: str) -> bool:
        self.reads.append(filename)
        return (
            filename in self.expected_reads
            and self.expected_reads[filename] != FILE_NOT_FOUND
        )
