import hashlib
import json
import os
from pathlib import Path
import re
import shutil
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
    if not isinstance(tools, dict):
        fail("Expected a Rustup toolchain table")
    selection = tools.get("path", tools.get("channel"))
    if not isinstance(selection, str) or not selection.strip():
        fail("Expected a Rustup channel or toolchain path")
    if "path" in tools:
        selection = str(Path(selection).resolve())
    if not matches_channel(os.environ.get("RUSTUP_TOOLCHAIN", selection), selection):
        fail("RUSTUP_TOOLCHAIN conflicts with rust-toolchain.toml")
    return config, selection


def matches_channel(name, channel):
    return name == channel or (
        not Path(channel).is_absolute() and name.startswith(channel + "-")
    )


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


def reconcile_rust(config):
    environment = dict(os.environ)
    environment.pop("RUSTUP_TOOLCHAIN", None)
    # Rustup reads profile, components and targets from the checkout itself.
    command = ["rustup", "toolchain", "install", "--no-update", "--no-self-update"]
    if "path" in config["toolchain"]:
        command = ["rustup", "show", "active-toolchain"]
    subprocess.run(command, env=environment, check=True)


def verify_rust(selection):
    active = Path(output("rustup", "which", "rustc")).resolve()
    if Path(selection).is_absolute():
        selected = (Path(selection) / "bin" / "rustc").resolve()
    else:
        selected = Path(
            output("rustup", "which", "rustc", "--toolchain", selection)
        ).resolve()
    expected = output(str(selected), "--version")
    if (
        active != selected
        or not expected.startswith("rustc ")
        or output("rustc", "--version") != expected
    ):
        fail("Active Rust compiler does not match rust-toolchain.toml")


def llvm_matches():
    rustc = output("rustc", "-vV")
    try:
        clang = output("clang", "--version")
        clangxx = output("clang++", "--version")
        linker = output("ld.lld", "--version")
    except FileNotFoundError:
        return False
    versions = [
        re.search(r"^LLVM version: (\d+)\.", rustc, re.MULTILINE),
        re.search(r"clang version (\d+)\.", clang),
        re.search(r"clang version (\d+)\.", clangxx),
        re.search(r"(?:^|\s)LLD (\d+)\.", linker),
    ]
    return all(versions) and len({match[1] for match in versions}) == 1


def provision_compilers():
    if not system_setup_allowed():
        fail("Compiler installation requires explicit system setup permission")
    version = re.search(r"^LLVM version: (\d+)\.", output("rustc", "-vV"), re.MULTILINE)
    if not version:
        fail("Cannot determine the active compiler's LLVM major")
    major = version[1]
    subprocess.run(["sudo", "apt-get", "update"], check=True)
    subprocess.run(
        [
            "sudo",
            "apt-get",
            "install",
            "--assume-yes",
            "--no-install-recommends",
            "build-essential",
            "ca-certificates",
            "curl",
            "git",
            "gnupg",
            "lsb-release",
            "wget",
            "libpq-dev",
            "libssl-dev",
            "lsof",
            "pkg-config",
            "software-properties-common",
        ],
        check=True,
    )
    script = Path(os.environ["RUNNER_TEMP"]) / "aptos-llvm.sh"
    subprocess.run(
        ["wget", "-O", str(script), "https://apt.llvm.org/llvm.sh"], check=True
    )
    subprocess.run(["sudo", "bash", str(script), major], check=True)
    for tool in ("clang", "clang++", "lld", "ld.lld"):
        subprocess.run(
            ["sudo", "update-alternatives", "--remove-all", tool], check=False
        )
        subprocess.run(
            [
                "sudo",
                "update-alternatives",
                "--install",
                f"/usr/bin/{tool}",
                tool,
                f"/usr/bin/{tool}-{major}",
                "100",
            ],
            check=True,
        )
    if not llvm_matches():
        fail("Provisioned LLVM tools do not match the active compiler")


def cache_identity():
    if not llvm_matches():
        fail("Finalize compiler provisioning before selecting the cache")
    identity = hashlib.sha256()
    for tool, args in (
        ("rustc", ("-vV",)),
        ("cargo", ("--version",)),
        ("clang", ("--version",)),
        ("clang++", ("--version",)),
        ("ld.lld", ("--version",)),
    ):
        identity.update(tool.encode() + b"\0" + output(tool, *args).encode() + b"\0")
        executable = shutil.which(tool)
        if executable is None:
            fail(f"Missing compiler tool: {tool}")
        identity.update(Path(executable).resolve().read_bytes())
    identity.update(Path("scripts/dev_setup.sh").read_bytes())
    with Path(os.environ["GITHUB_ENV"]).open("a") as stream:
        stream.write("APTOS_BUILD_ENV=" + identity.hexdigest() + "\n")


def system_setup_allowed():
    return (
        os.environ.get("RUNNER_ENVIRONMENT") == "github-hosted"
        or os.environ.get("ALLOW_SYSTEM_SETUP") == "true"
    )


def select_system_setup(reason):
    # Self-hosted installation is an explicit capability of the isolated job.
    if not system_setup_allowed():
        fail(f"{reason}; use a compatible runner image or select the AWS provider")
    print(f"::warning::{reason}; running normal Rust setup", flush=True)
    with Path(os.environ["GITHUB_OUTPUT"]).open("a") as stream:
        stream.write("system-setup=true\n")


def main(mode):
    if mode == "prepare":
        reason = None
        if MANIFEST.exists():
            if "scripts/dev_setup.sh" in stale_files():
                reason = "Prebuilt system-tool setup differs from the checkout"
        else:
            reason = "No prebuilt Rust image manifest"
        config, selection = toolchain()
        if reason and not system_setup_allowed():
            select_system_setup(reason)
        reconcile_rust(config)
        verify_rust(selection)
        if reason:
            select_system_setup(reason)
        elif not llvm_matches():
            select_system_setup(
                "Prebuilt LLVM tools do not match the active Rust compiler"
            )
    elif mode == "compilers":
        provision_compilers()
    elif mode == "cache-identity":
        cache_identity()
    elif mode == "verify":
        config, selection = toolchain()
        reconcile_rust(config)
        verify_rust(selection)
        if not llvm_matches():
            fail(
                "LLVM compiler/linker do not match the active Rust compiler after setup"
            )
    else:
        fail("Unknown setup phase")


if __name__ == "__main__":
    main(sys.argv[1])
