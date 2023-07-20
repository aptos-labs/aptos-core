# A wrapper around filesystem operations

from __future__ import annotations

import os
import resource
import tempfile
import unittest
import shutil
from typing import Dict, List, Optional


class Filesystem:
    def write(self, filename: str, contents: bytes) -> None:
        raise NotImplementedError()

    def read(self, filename: str) -> bytes:
        raise NotImplementedError()

    def mkstemp(self) -> str:
        raise NotImplementedError()

    def mkdtemp(self) -> str:
        raise NotImplementedError()

    def mkdir(self, foldername: str) -> None:
        raise NotImplementedError()

    def rmtree(self, foldername: str) -> None:
        raise NotImplementedError()

    def copyfile(self, copy_from: str, copy_to: str) -> None:
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

    def mkdtemp(self) -> str:
        return tempfile.mkdtemp()

    def mkdir(self, foldername: str) -> None:
        os.mkdir(foldername)

    def rmtree(self, foldername: str) -> None:
        shutil.rmtree(foldername)

    def copyfile(self, copy_from: str, copy_to: str) -> None:
        shutil.copyfile(copy_from, copy_to)

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

    def mkdtemp(self) -> str:
        return "fake"

    def mkdir(self, foldername: str) -> None:
        return

    def rmtree(self, foldername: str) -> None:
        return

    def copyfile(self, copy_from: str, copy_to: str) -> None:
        return

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
        self.writes: Dict[str, bytes] = {}
        self.reads: List[str] = []
        self.temp_count = 1
        self.unlinks: List[str] = []

    def write(self, filename: str, contents: bytes) -> None:
        self.writes[filename] = contents

    def get_write(self, filename: str) -> bytes:
        return self.writes[filename]

    def read(self, filename: str) -> bytes:
        self.reads.append(filename)
        return self.expected_reads.get(filename, b"")

    def assert_writes(self, testcase: unittest.TestCase) -> None:
        for filename, contents in self.expected_writes.items():
            testcase.assertIn(
                filename, self.writes, f"{filename} was not written: {self.writes}"
            )
            testcase.assertMultiLineEqual(
                self.writes[filename].decode(),
                contents.decode(),
                f"{filename} did not match expected contents",
            )

    def assert_reads(self, testcase: unittest.TestCase) -> None:
        for filename, _content in self.expected_reads.items():
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

    def mkdtemp(self) -> str:
        foldername = f"temp_folder{self.temp_count}"
        self.temp_count += 1
        return foldername

    def mkdir(self, foldername: str) -> None:
        self.writes[foldername] = b""

    def rmtree(self, foldername: str) -> None:
        for path in self.writes:
            if path.startswith(foldername):
                self.writes[path]

    def copyfile(self, copy_from: str, copy_to: str) -> None:
        if copy_from in self.writes:
            self.writes[copy_to] = self.writes[copy_from]

    def unlink(self, filename: str) -> None:
        self.unlinks.append(filename)

    def assert_unlinks(self, testcase: unittest.TestCase) -> None:
        for filename in self.expected_unlinks:
            testcase.assertIn(filename, self.unlinks, f"{filename} was not unlinked")

    def exists(self, filename: str) -> bool:
        self.reads.append(filename)
        return (
            filename in self.expected_reads
            and self.expected_reads[filename] != FILE_NOT_FOUND
        )
