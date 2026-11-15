import json
import os
from pathlib import Path
import subprocess
import sys

ROOT = Path("cache-fixture")


def prepare():
    (ROOT / "src").mkdir(parents=True, exist_ok=True)
    (ROOT / "Cargo.toml").write_text(
        '[package]\nname="cache-contract"\nversion="0.1.0"\nedition="2021"\n'
        '[workspace]\n[dependencies]\nitoa="=1.0.15"\n'
    )
    (ROOT / "src/main.rs").write_text(
        'fn main(){let mut b=itoa::Buffer::new();println!("{}",b.format(42));}\n'
    )
    subprocess.run(["cargo", "generate-lockfile"], cwd=ROOT, check=True)


def build(environment=None):
    result = subprocess.run(
        ["cargo", "build", "--locked", "--message-format=json"],
        cwd=ROOT,
        text=True,
        capture_output=True,
        check=True,
        env=environment,
    )
    artifacts = [json.loads(line) for line in result.stdout.splitlines()]
    external = [
        m
        for m in artifacts
        if m.get("reason") == "compiler-artifact"
        and m["package_id"].startswith("registry+")
    ]
    assert external, "Representative dependency did not compile"
    assert (
        subprocess.check_output(
            [str(ROOT / "target/debug/cache-contract")], text=True
        ).strip()
        == "42"
    )
    return external


def run(phase):
    if phase == "warm":
        assert os.environ["CACHE_HIT"] == "true", (
            "Separate job did not restore exact cache"
        )
        assert not (ROOT / "target/debug/cache-contract").exists(), (
            "Workspace output should not be cached"
        )
    else:
        assert os.environ.get("CACHE_HIT") != "true", "Cold epoch already existed"
    external = build()
    assert all(m["fresh"] == (phase == "warm") for m in external)
    source = ROOT / "src/main.rs"
    source.write_text(source.read_text() + "\n// source transition\n")
    assert all(m["fresh"] for m in build()), "Source changes should retain dependencies"
    flags = dict(os.environ, RUSTFLAGS="-C opt-level=1")
    assert all(not m["fresh"] for m in build(flags)), (
        "Flag changes must rebuild dependencies"
    )
    print(
        json.dumps({"phase": phase, "external_fresh": [m["fresh"] for m in external]})
    )


if __name__ == "__main__":
    if sys.argv[1] == "prepare":
        prepare()
    else:
        run(sys.argv[1])
