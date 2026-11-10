import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import sys
import tomllib

MANIFEST = Path("/opt/aptos-rust-image.json")
CONTRACT_FILES = {"rust-toolchain.toml", "scripts/dev_setup.sh"}


def fail(message):
    raise SystemExit(f"::error::{message}")


def output(*args):
    return subprocess.check_output(args, text=True).strip()


def toolchain():
    config = tomllib.loads(Path("rust-toolchain.toml").read_text())
    tools = config.get("toolchain")
    if (
        set(config) != {"toolchain"}
        or not isinstance(tools, dict)
        or set(tools) != {"channel", "components"}
    ):
        fail("Expected only a pinned Rust channel and components")
    channel, components = tools["channel"], tools["components"]
    if not isinstance(channel, str) or not re.fullmatch(r"\d+\.\d+\.\d+", channel):
        fail("Rust channel must pin a release version")
    if (
        not isinstance(components, list)
        or not components
        or any(
            not isinstance(c, str) or not re.fullmatch(r"[a-z][a-z0-9-]*", c)
            for c in components
        )
    ):
        fail("Invalid Rust toolchain components")
    if not matches_channel(os.environ.get("RUSTUP_TOOLCHAIN", channel), channel):
        fail("RUSTUP_TOOLCHAIN conflicts with rust-toolchain.toml")
    return channel, components


def matches_channel(name, channel):
    return name == channel or name.startswith(channel + "-")


def stale_files():
    manifest = json.loads(MANIFEST.read_text())
    if not isinstance(manifest, dict) or set(manifest) != {
        "files",
        "source_sha",
        "rustc",
        "clang",
    }:
        fail("Invalid image manifest fields")
    files = manifest["files"]
    if not isinstance(files, dict) or set(files) != CONTRACT_FILES:
        fail(
            "Image manifest must record exactly rust-toolchain.toml and scripts/dev_setup.sh"
        )
    if any(
        not isinstance(value, str) or not re.fullmatch(r"[0-9a-f]{64}", value)
        for value in files.values()
    ):
        fail("Invalid image manifest SHA256 digest")
    if not isinstance(manifest["source_sha"], str) or not re.fullmatch(
        r"[0-9a-fA-F]{40}", manifest["source_sha"]
    ):
        fail("Invalid image manifest source_sha")
    if any(
        not isinstance(manifest[name], str) or not manifest[name].strip()
        for name in ("rustc", "clang")
    ):
        fail("Image manifest must record compiler versions")
    return {
        name
        for name in CONTRACT_FILES
        if hashlib.sha256(Path(name).read_bytes()).hexdigest() != files[name]
    }


def reconcile_rust(channel, components):
    installed = [
        line.split()[0]
        for line in output("rustup", "toolchain", "list").splitlines()
        if line.strip()
    ]
    if not any(matches_channel(name, channel) for name in installed):
        subprocess.run(["rustup", "toolchain", "install", channel], check=True)
    subprocess.run(
        ["rustup", "component", "add", "--toolchain", channel, *components], check=True
    )


def verify_rust(channel):
    active = output("rustup", "show", "active-toolchain").split()[0]
    expected = output("rustup", "run", channel, "rustc", "--version")
    if (
        not matches_channel(active, channel)
        or not expected.startswith(f"rustc {channel} ")
        or output("rustc", "--version") != expected
    ):
        fail("Active Rust compiler does not match rust-toolchain.toml")


def llvm_matches():
    versions = [
        re.search(r"^LLVM version: (\d+)\.", output("rustc", "-vV"), re.MULTILINE),
        re.search(r"clang version (\d+)\.", output("clang", "--version")),
        re.search(r"^LLD (\d+)\.", output("ld.lld", "--version")),
    ]
    return all(versions) and len({match[1] for match in versions}) == 1


def select_system_setup(reason):
    # Only GitHub-managed disposable VMs may install system dependencies.
    if os.environ.get("RUNNER_ENVIRONMENT") != "github-hosted":
        fail(f"{reason}; use a compatible runner image or select the AWS provider")
    print(f"::warning::{reason}; running normal Rust setup", flush=True)
    with Path(os.environ["GITHUB_OUTPUT"]).open("a") as stream:
        stream.write("system-setup=true\n")


def main(mode):
    if mode == "prepare":
        if MANIFEST.exists():
            stale = stale_files()
            channel, components = toolchain()
            if "scripts/dev_setup.sh" in stale:
                select_system_setup(
                    "Prebuilt system-tool setup differs from the checkout"
                )
                return
            reconcile_rust(channel, components)
            verify_rust(channel)
            if not llvm_matches():
                select_system_setup(
                    "Prebuilt LLVM tools do not match the active Rust compiler"
                )
        else:
            toolchain()
            select_system_setup("No prebuilt Rust image manifest")
    elif mode == "verify":
        channel, components = toolchain()
        reconcile_rust(channel, components)
        verify_rust(channel)
        if not llvm_matches():
            fail(
                "LLVM compiler/linker do not match the active Rust compiler after setup"
            )
    else:
        fail("Unknown setup phase")


if __name__ == "__main__":
    main(sys.argv[1])
