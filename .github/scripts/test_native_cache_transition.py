import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]


@unittest.skipUnless(
    shutil.which("sha256sum") and shutil.which("clang") and shutil.which("ld.lld"),
    "Linux compiler tools required",
)
class NativeCacheTests(unittest.TestCase):
    def test_same_path_compiler_change_and_older_source(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "src").mkdir()
            (root / "Cargo.toml").write_text(
                '[package]\nname="native-cache-check"\nversion="0.1.0"\nedition="2021"\n[workspace]\n'
            )
            (root / "build.rs").write_text(
                'use std::{env,process::Command};fn main(){let out=env::var("OUT_DIR").unwrap();assert!(Command::new(env::var("CC").unwrap()).args(["-c","value.c","-o",&format!("{out}/value.o")]).status().unwrap().success());assert!(Command::new("ar").args(["crs",&format!("{out}/libvalue.a"),&format!("{out}/value.o")]).status().unwrap().success());println!("cargo:rustc-link-search=native={out}");println!("cargo:rustc-link-lib=static=value");println!("cargo:rerun-if-changed=value.c");}\n'
            )
            source = root / "value.c"
            source.write_text("int value(void){return PROBE_VALUE;}\n")
            (root / "src/main.rs").write_text(
                'extern "C" {fn value()->i32;}fn main(){println!("{}",unsafe{value()});}\n'
            )
            wrapper = root / "compiler"
            compiler = shutil.which("clang")

            def select(value):
                wrapper.write_text(
                    f'#!/bin/sh\nexec "{compiler}" -DPROBE_VALUE={value} "$@"\n'
                )
                wrapper.chmod(0o755)

            env = dict(
                os.environ,
                CC=str(wrapper),
                CARGO_TARGET_DIR="target",
                CARGO_INCREMENTAL="0",
            )

            def build(expected):
                command = 'source "$1"; cargo build --offline; "$CARGO_TARGET_DIR/debug/native-cache-check"'
                result = subprocess.run(
                    [
                        "bash",
                        "-euc",
                        command,
                        "probe",
                        str(ROOT / "docker/builder/refresh-source-cache.sh"),
                    ],
                    cwd=root,
                    env=env,
                    text=True,
                    capture_output=True,
                )
                self.assertEqual(result.returncode, 0, result.stderr)
                self.assertEqual(result.stdout.strip(), str(expected))

            select(1)
            build(1)
            build(1)
            select(2)
            build(2)
            source.write_text("int value(void){return 3;}\n")
            os.utime(source, (1, 1))
            build(3)


if __name__ == "__main__":
    unittest.main()
