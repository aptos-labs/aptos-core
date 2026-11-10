import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

import yaml

ROOT = Path(__file__).resolve().parents[2]
ACTION = ROOT / ".github/actions/rust-prebuilt-setup"

STUB = r"""import json, os, pathlib, sys
name, args = pathlib.Path(sys.argv[0]).name, sys.argv[1:]
root = pathlib.Path(os.environ['FIXTURE_ROOT'])
state_path = root / 'state.json'
state = json.loads(state_path.read_text())
with (root / 'calls.jsonl').open('a') as log:
    log.write(json.dumps([name, args]) + '\n')
if os.environ.get('FAIL_TOOL') == name:
    raise SystemExit(1)
channel = state['channel']
if name == 'rustup':
    if args == ['toolchain', 'list']:
        print(channel + '-x86_64-unknown-linux-gnu' if state['installed'] else 'stable-x86_64-unknown-linux-gnu')
    elif args[:2] == ['toolchain', 'install']:
        if os.environ.get('FAIL_INSTALL'):
            raise SystemExit(1)
        assert args[2:] == [channel], args
        state['installed'] = True
    elif args[:2] == ['component', 'add']:
        if os.environ.get('FAIL_COMPONENT'):
            raise SystemExit(1)
        assert args[2:4] == ['--toolchain', channel], args
        state['components'] = sorted(set(state['components']) | set(args[4:]))
    elif args == ['show', 'active-toolchain']:
        print(os.environ.get('ACTIVE_CHANNEL', channel) + '-x86_64-unknown-linux-gnu (directory override)')
    elif args == ['run', channel, 'rustc', '--version']:
        print('rustc ' + channel + ' (fixture)')
    else:
        raise SystemExit('unexpected rustup invocation: ' + repr(args))
    state_path.write_text(json.dumps(state))
elif name == 'rustc':
    assert state['installed']
    if args == ['-vV']:
        print('LLVM version: ' + str(state['rust_llvm']) + '.1.0')
    else:
        print('rustc ' + os.environ.get('COMPILER_CHANNEL', channel) + ' (fixture)')
elif name == 'clang':
    print('Ubuntu clang version ' + str(state['clang_llvm']) + '.1.0')
elif name == 'ld.lld':
    print(os.environ.get('LLD_PREFIX', 'Ubuntu ') + 'LLD ' + str(state['lld_llvm']) + '.1.0 (compatible with GNU linkers)')
else:
    print(name + ' fixture')
"""


class PrebuiltSetupTests(unittest.TestCase):
    def setUp(self):
        temp = tempfile.TemporaryDirectory(prefix="aptos-prebuilt-test-")
        self.addCleanup(temp.cleanup)
        self.root = Path(temp.name)
        self.home = self.root / "home"
        for path in ["bin", ".cargo/bin", ".dotnet/tools", ".local/bin"]:
            (self.home / path).mkdir(parents=True)
        (self.root / "scripts").mkdir()
        self.components = ["cargo", "clippy", "rustc"]
        self.set_toolchain("1.99.1")
        (self.root / "scripts/dev_setup.sh").write_text("fixture setup\n")
        self.manifest = {
            "files": {
                name: hashlib.sha256((self.root / name).read_bytes()).hexdigest()
                for name in ["rust-toolchain.toml", "scripts/dev_setup.sh"]
            },
            "source_sha": "a" * 40,
            "rustc": "rustc recorded-image-version",
            "clang": "clang recorded-image-version",
        }
        self.manifest_path = self.root / "manifest.json"
        self.manifest_path.write_text(json.dumps(self.manifest))
        self.set_state(
            channel="1.99.1",
            installed=True,
            components=self.components,
            rust_llvm=23,
            clang_llvm=23,
            lld_llvm=23,
        )
        for name in ["github-path", "github-env", "github-output"]:
            (self.root / name).touch()
        stub = self.root / "stub"
        stub.write_text(f"#!{sys.executable}\n" + STUB)
        stub.chmod(0o755)
        for name in [
            "rustup",
            "rustc",
            "cargo",
            "clang",
            "ld.lld",
            "protoc",
            "pre-commit",
            "docker",
            "z3",
            "cvc5",
        ]:
            (self.home / "bin" / name).symlink_to(stub)
        (self.home / ".dotnet/tools/boogie").symlink_to(stub)
        (self.root / "setup.py").write_text(
            (ACTION / "setup.py")
            .read_text()
            .replace(
                'Path("/opt/aptos-rust-image.json")',
                f"Path({str(self.manifest_path)!r})",
            )
        )
        self.action = yaml.load(
            (ACTION / "action.yaml").read_text(), Loader=yaml.BaseLoader
        )
        self.steps = self.action["runs"]["steps"]
        self.env = {
            "PATH": os.environ["PATH"],
            "HOME": str(self.home),
            "FIXTURE_ROOT": str(self.root),
            "GITHUB_ACTION_PATH": str(self.root),
            "GITHUB_PATH": str(self.root / "github-path"),
            "GITHUB_ENV": str(self.root / "github-env"),
            "GITHUB_OUTPUT": str(self.root / "github-output"),
            "RUNNER_ENVIRONMENT": "self-hosted",
            "PYTHONDONTWRITEBYTECODE": "1",
        }

    def set_toolchain(self, channel):
        (self.root / "rust-toolchain.toml").write_text(
            f'[toolchain]\nchannel = "{channel}"\ncomponents = {json.dumps(self.components)}\n'
        )

    def set_state(self, **changes):
        path = self.root / "state.json"
        state = json.loads(path.read_text()) if path.exists() else {}
        path.write_text(json.dumps(dict(state, **changes)))

    def run_step(self, index, **env):
        script = self.steps[index]["run"].replace(
            'test "$HOME" = /home/runner', f'test "$HOME" = "{self.home}"'
        )
        return subprocess.run(
            ["bash", "-c", script],
            cwd=self.root,
            env=dict(self.env, **env),
            text=True,
            capture_output=True,
        )

    def calls(self):
        path = self.root / "calls.jsonl"
        return (
            [json.loads(line) for line in path.read_text().splitlines()]
            if path.exists()
            else []
        )

    def assert_no_exports(self):
        for name in ["github-path", "github-env"]:
            self.assertEqual((self.root / name).read_text(), "")

    def test_matching_image_avoids_full_setup_and_verifies_before_cache(self):
        result = self.run_step(0)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual((self.root / "github-output").read_text(), "")
        self.assert_no_exports()
        self.assertEqual(self.run_step(2).returncode, 0)
        self.assertEqual(self.calls()[-1], ["docker", ["info"]])
        self.assertIn(
            str(self.home / ".cargo/bin"), (self.root / "github-path").read_text()
        )
        self.assertEqual(
            self.steps[-1]["with"]["save-if"], "${{ inputs.save-cache == 'true' }}"
        )
        self.assertIn("Swatinem/rust-cache@", self.steps[-1]["uses"])
        self.assertEqual(self.steps[1]["uses"], "./.github/actions/rust-setup")
        self.assertEqual(self.steps[1]["with"]["SKIP_CACHE"], "true")
        self.assertEqual(
            self.steps[1]["if"], "${{ steps.setup.outputs.system-setup == 'true' }}"
        )
        self.assertEqual(
            self.steps[0]["env"]["RUNNER_ENVIRONMENT"], "${{ runner.environment }}"
        )
        normal = yaml.load(
            (ROOT / ".github/actions/rust-setup/action.yaml").read_text(),
            Loader=yaml.BaseLoader,
        )
        cache = next(
            s for s in normal["runs"]["steps"] if "rust-cache@" in s.get("uses", "")
        )
        self.assertEqual(cache["if"], "${{ inputs.SKIP_CACHE != 'true' }}")
        self.assertEqual(normal["inputs"]["SKIP_CACHE"]["default"], "false")

    def test_full_setup_replaces_old_linker_alternatives_before_install(self):
        normal = yaml.load(
            (ROOT / ".github/actions/rust-setup/action.yaml").read_text(),
            Loader=yaml.BaseLoader,
        )
        script = normal["runs"]["steps"][0]["run"]
        commands = self.root / "system-commands.jsonl"
        sudo = self.home / "bin/sudo"
        sudo.write_text(
            f"#!{sys.executable}\nimport json,sys\n"
            f"with open({str(commands)!r}, 'a') as log:\n"
            "    log.write(json.dumps(sys.argv[1:]) + '\\n')\n"
        )
        sudo.chmod(0o755)
        wget = self.home / "bin/wget"
        wget.write_text("#!/bin/sh\nprintf '# fixture installer\\n'\n")
        wget.chmod(0o755)
        result = subprocess.run(
            ["bash", "-e", "-c", script],
            cwd=self.root,
            env=dict(self.env, PATH=f"{self.home}/bin:{os.environ['PATH']}"),
            capture_output=True,
            text=True,
        )
        self.assertEqual(result.returncode, 0, result.stderr)
        calls = [json.loads(line) for line in commands.read_text().splitlines()]
        for tool in ["clang", "clang++", "lld", "ld.lld"]:
            removed = calls.index(["update-alternatives", "--remove-all", tool])
            installed = next(
                i
                for i, args in enumerate(calls)
                if args[:3] == ["update-alternatives", "--install", f"/usr/bin/{tool}"]
            )
            self.assertLess(removed, installed)

    def test_rust_drift_repairs_channel_and_components_without_system_setup(self):
        self.set_toolchain("1.99.2")
        self.set_state(channel="1.99.2", installed=False, components=[])
        result = self.run_step(0)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn(["rustup", ["toolchain", "install", "1.99.2"]], self.calls())
        self.assertEqual((self.root / "github-output").read_text(), "")
        self.assertEqual(self.run_step(2).returncode, 0)

    def test_upstream_and_ubuntu_linker_version_formats_are_accepted(self):
        for prefix in ["", "Ubuntu "]:
            with self.subTest(prefix=prefix):
                result = self.run_step(0, LLD_PREFIX=prefix)
                self.assertEqual(result.returncode, 0, result.stderr)

    def test_installed_channel_repairs_missing_components(self):
        self.set_state(components=["rustc"])
        self.assertEqual(self.run_step(0).returncode, 0)
        state = json.loads((self.root / "state.json").read_text())
        self.assertEqual(set(state["components"]), set(self.components))
        self.assertFalse(
            any(args[:2] == ["toolchain", "install"] for _, args in self.calls())
        )

    def test_system_drift_selects_hosted_setup_then_requires_verified_tools(self):
        (self.root / "scripts/dev_setup.sh").write_text("changed setup\n")
        self.set_state(clang_llvm=22, lld_llvm=22)
        result = self.run_step(0, RUNNER_ENVIRONMENT="github-hosted")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(self.calls(), [])
        self.assertEqual(
            (self.root / "github-output").read_text(), "system-setup=true\n"
        )
        self.assertNotEqual(self.run_step(2).returncode, 0)
        self.assert_no_exports()
        self.set_state(clang_llvm=23, lld_llvm=23)
        result = self.run_step(2)
        self.assertEqual(result.returncode, 0, result.stderr)

    def test_system_drift_requires_explicit_self_hosted_installation_capability(self):
        (self.root / "scripts/dev_setup.sh").write_text("changed setup\n")
        result = self.run_step(0)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("compatible runner image", result.stderr)
        self.assertEqual(self.calls(), [])
        self.assertEqual((self.root / "github-output").read_text(), "")

    def test_isolated_self_hosted_job_can_opt_into_normal_setup(self):
        (self.root / "scripts/dev_setup.sh").write_text("changed setup\n")
        for value in ["false", "", "yes", "1"]:
            with self.subTest(value=value):
                self.assertNotEqual(
                    self.run_step(0, ALLOW_SYSTEM_SETUP=value).returncode, 0
                )
                self.assertEqual(self.calls(), [])
        result = self.run_step(0, ALLOW_SYSTEM_SETUP="true")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(
            (self.root / "github-output").read_text(), "system-setup=true\n"
        )
        self.assertEqual(
            self.action["inputs"]["allow-system-setup"]["default"], "false"
        )
        self.assertEqual(
            self.steps[0]["env"]["ALLOW_SYSTEM_SETUP"],
            "${{ inputs.allow-system-setup }}",
        )

    def test_self_hosted_opt_in_does_not_skip_compiler_verification(self):
        self.set_state(lld_llvm=22)
        self.assertEqual(self.run_step(0, ALLOW_SYSTEM_SETUP="true").returncode, 0)
        self.assertNotEqual(self.run_step(2, ALLOW_SYSTEM_SETUP="true").returncode, 0)
        self.assert_no_exports()

    def test_llvm_drift_routes_only_hosted_and_checks_linker_independently(self):
        for key in ["clang_llvm", "lld_llvm"]:
            with self.subTest(key=key):
                self.set_state(**dict({"clang_llvm": 23, "lld_llvm": 23}, **{key: 22}))
                self.assertNotEqual(self.run_step(0).returncode, 0)
                result = self.run_step(0, RUNNER_ENVIRONMENT="github-hosted")
                self.assertEqual(result.returncode, 0, result.stderr)
                self.assertIn(
                    "system-setup=true", (self.root / "github-output").read_text()
                )
                (self.root / "github-output").write_text("")
                self.assert_no_exports()

    def test_missing_manifest_is_cold_setup_only_on_hosted(self):
        self.manifest_path.unlink()
        self.assertNotEqual(self.run_step(0).returncode, 0)
        self.assertEqual(self.calls(), [])
        self.assertEqual(
            self.run_step(0, RUNNER_ENVIRONMENT="github-hosted").returncode, 0
        )
        self.assertEqual(
            (self.root / "github-output").read_text(), "system-setup=true\n"
        )

    def test_malformed_manifest_never_triggers_fallback(self):
        cases = [
            None,
            {},
            dict(self.manifest, files={}),
            dict(self.manifest, source_sha="bad"),
            dict(self.manifest, files={"../unexpected": "a" * 64}),
            dict(
                self.manifest,
                files=dict(self.manifest["files"], **{"rust-toolchain.toml": "x"}),
            ),
            dict(self.manifest, clang=""),
        ]
        for manifest in cases:
            with self.subTest(manifest=manifest):
                self.manifest_path.write_text(json.dumps(manifest))
                result = self.run_step(0, RUNNER_ENVIRONMENT="github-hosted")
                self.assertNotEqual(result.returncode, 0)
                self.assertEqual(self.calls(), [])
                self.assertEqual((self.root / "github-output").read_text(), "")

    def test_conflicting_override_and_incorrect_active_compiler_fail(self):
        result = self.run_step(0, RUSTUP_TOOLCHAIN="stable")
        self.assertNotEqual(result.returncode, 0)
        self.assertEqual(self.calls(), [])
        for env in [{"ACTIVE_CHANNEL": "stable"}, {"COMPILER_CHANNEL": "1.97.0"}]:
            with self.subTest(env=env):
                self.assertNotEqual(self.run_step(0, **env).returncode, 0)
                self.assertEqual((self.root / "github-output").read_text(), "")
                self.assert_no_exports()
        self.assertEqual(
            self.run_step(
                0, RUSTUP_TOOLCHAIN="1.99.1-x86_64-unknown-linux-gnu"
            ).returncode,
            0,
        )

    def test_install_component_and_tool_failures_stop_before_exports(self):
        for env in [
            {"FAIL_INSTALL": "1"},
            {"FAIL_COMPONENT": "1"},
            {"FAIL_TOOL": "docker"},
        ]:
            with self.subTest(env=env):
                self.set_state(installed=not bool(env.get("FAIL_INSTALL")))
                self.assertNotEqual(self.run_step(2, **env).returncode, 0)
                self.assert_no_exports()

    def test_unsupported_toolchain_fields_fail_before_install(self):
        with (self.root / "rust-toolchain.toml").open("a") as stream:
            stream.write('targets = ["x86_64-unknown-linux-gnu"]\n')
        self.assertNotEqual(self.run_step(0).returncode, 0)
        self.assertEqual(self.calls(), [])


if __name__ == "__main__":
    unittest.main()
