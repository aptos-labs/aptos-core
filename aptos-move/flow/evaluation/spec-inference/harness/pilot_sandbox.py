"""Bubblewrap policy for versioned Phase 4 controller rounds.

The wrapper deliberately isolates the whole controller, not just Claude Code.
Only the selected run manifest, shared package, task patch, plugin, evaluation
harness, and a private per-run output staging directory are mounted. The staging directory
is atomically published after the child exits, so a run cannot inspect sibling
run artifacts through the shared artifacts directory.
"""

from __future__ import annotations

import argparse
import asyncio
import json
import os
import shlex
import contextlib
import shutil
import signal
import subprocess
import sys
import tempfile
from dataclasses import dataclass
from pathlib import Path
from typing import Iterable

from .artifacts import sha256_file
from .boogie_proxy import BoogieProxy
from .config import ExperimentConfig, RunSpec


POLICY_VERSION = 3
# Landlock confines the agent process itself to a subset of what the sandbox
# mounts, so the outer namespace and the inner ruleset are two independent
# layers rather than one repeated.
LANDLOCK_SOURCE = "landlock_exec.c"
LANDLOCK_EXEC = "landlock-exec"
# `/proc` is mounted read-only because Boogie needs it: its self-contained
# CoreCLR reads `/proc/self` at startup and aborts with `HRESULT: 0x8007000E`
# without it. Only the controller runs Boogie. The agent's ruleset names
# `/proc/self`, which Landlock resolves to the wrapper's own PID and to no
# child's, so an agent-spawned Boogie could not start; the agent therefore
# reaches Boogie through the controller's proxy (`boogie_proxy.py`), and
# preflight proves that path as the grandchild it really is.
AGENT_PROC_PATHS = (Path("/proc/self"), Path("/proc/stat"), Path("/proc/sys/vm"))
#: Where the agent finds Boogie: the proxy client, never the executable. See
#: `boogie_proxy.py` for why the agent's own Landlock domain cannot run it.
AGENT_BOOGIE = Path("/opt/bin/boogie")
BOOGIE_PROXY_CLIENT = "boogie-proxy-client.py"
BOOGIE_PROXY_SOCKET = Path("/tmp/move-inference-boogie.sock")
SYSTEM_READ_ONLY = (Path("/usr"), Path("/lib"))
RESOLVER_INPUTS = (
    Path("/etc/hosts"),
    Path("/etc/nsswitch.conf"),
    Path("/etc/resolv.conf"),
    Path("/etc/gai.conf"),
)


@dataclass(frozen=True)
class Launch:
    command: tuple[str, ...]
    config: Path
    run_manifest: Path
    artifacts: Path
    run_id: str
    shared_package: Path
    task_patch: Path
    plugin: Path
    evaluation_root: Path
    python_root: Path | None
    move_flow: Path
    claude: Path
    boogie: Path
    boogie_client: Path
    z3: Path
    landlock: Path
    feedback_level: str
    #: Mutants the controller refutes an accepted contract against, mounted for
    #: the controller and withheld from the agent by its Landlock ruleset.
    refutation_mutants: Path | None = None


def main() -> None:
    if sys.argv[1:] == ["--preflight"]:
        result = preflight()
        print(json.dumps(result, sort_keys=True))
        if not result["passed"]:
            raise SystemExit(1)
        return
    launch = parse_launch(sys.argv[1:])
    raise SystemExit(run_isolated(launch))


def preflight() -> dict[str, object]:
    bwrap = shutil.which("bwrap")
    if bwrap is None:
        return _preflight_result(False, "bubblewrap is not installed", None)
    bwrap_path = Path(bwrap).resolve()
    evaluation_root = Path(__file__).resolve().parent.parent
    try:
        landlock = _required_landlock(evaluation_root)
    except SystemExit as error:
        return _preflight_result(False, str(error), bwrap_path)
    with tempfile.TemporaryDirectory(prefix="move-inference-sandbox-probe-") as temporary:
        sentinel = Path(temporary) / "must-not-be-visible"
        sentinel.write_text("host-only", encoding="utf-8")
        command = [
            str(bwrap_path),
            "--die-with-parent",
            "--new-session",
            "--unshare-user",
            "--unshare-pid",
            "--unshare-ipc",
            "--unshare-uts",
            "--uid",
            "0",
            "--gid",
            "0",
            "--cap-drop",
            "ALL",
            "--ro-bind",
            "/usr",
            "/usr",
            "--ro-bind",
            "/lib",
            "/lib",
            "--symlink",
            "usr/bin",
            "/bin",
            "--dev",
            "/dev",
            "--tmpfs",
            "/tmp",
            # `/proc` is mounted for Boogie, so the probe asserts that the
            # agent's own confinement hides another process rather than that
            # `/proc` is absent.
            "--ro-bind",
            "/proc",
            "/proc",
            "--ro-bind",
            str(landlock),
            "/opt/bin/landlock-exec",
            "--clearenv",
            "--setenv",
            "PATH",
            "/usr/bin:/bin",
            "/opt/bin/landlock-exec",
            "--ro",
            "/usr",
            "--ro",
            "/lib",
            "--ro",
            "/proc/self",
            "--rw",
            "/tmp",
            "--",
            "/usr/bin/python3",
            "-c",
            (
                # Landlock governs opening a file rather than stat-ing it, so
                # the probe reads rather than asks whether the path exists.
                "import os, pathlib\n"
                f"assert not pathlib.Path({str(sentinel)!r}).exists(), 'host path visible'\n"
                "pathlib.Path('/proc/self/status').read_text()\n"
                "try:\n"
                "    pathlib.Path('/proc/1/status').read_text()\n"
                "except OSError:\n"
                "    pass\n"
                "else:\n"
                "    raise SystemExit('another process is readable')\n"
                # Truncation is a distinct Landlock right. Ungoverned, a file
                # the ruleset keeps read-only can still be emptied -- the
                # pristine baseline and the run record among them.
                "try:\n"
                "    os.truncate('/usr/bin/env', 0)\n"
                "except OSError:\n"
                "    pass\n"
                "else:\n"
                "    raise SystemExit('a read-only file can be truncated')\n"
            ),
        ]
        process = subprocess.run(
            command,
            capture_output=True,
            text=True,
            timeout=30,
            check=False,
        )
    passed = process.returncode == 0
    detail = (
        f"policy={POLICY_VERSION}, bwrap={bwrap_path}, "
        f"sha256={sha256_file(bwrap_path)}, landlock={landlock}, "
        f"landlock_sha256={sha256_file(landlock)}, "
        f"host-path-and-agent-proc-isolation={'passed' if passed else 'failed'}"
    )
    diagnostics = (process.stderr or process.stdout).strip()
    if diagnostics:
        detail += f", diagnostics={diagnostics}"
    isolation = passed
    prover_pipeline: bool | None = None
    if isolation:
        pipeline_failure = _probe_prover_pipeline(bwrap_path, landlock, evaluation_root)
        prover_pipeline = pipeline_failure is None
        passed = prover_pipeline
        detail += (
            f", prover-pipeline-as-agent-grandchild={'passed' if prover_pipeline else 'failed'}"
        )
        if pipeline_failure:
            detail += f", diagnostics={pipeline_failure}"
    return _preflight_result(passed, detail, bwrap_path, isolation, prover_pipeline)


PROBE_MODULE = """module 0x42::probe {
    fun identity(x: u64): u64 { x }
    spec identity {
        ensures result == x;
    }
}
"""


def _probe_prover_pipeline(bwrap: Path, landlock: Path, evaluation_root: Path) -> str | None:
    """Prove a one-line package the way the agent's tools will, and require it.

    The agent never exec's a solver directly: Landlocked Claude starts
    `move-flow`, which probes Z3 itself and runs Boogie through the proxy
    client, and Boogie in turn starts Z3. A probe that exec's one tool from
    the wrapper passes where that chain fails, so this runs the actual
    `move-flow experiment prove` from a shell under the agent's own ruleset,
    with the proxy served from this process exactly as the controller serves
    it, and requires the report to say the obligation was proved.
    """
    try:
        boogie = _required_solver("BOOGIE_EXE", "boogie")
        z3 = _required_solver("Z3_EXE", "z3")
        move_flow = _required_executable("move-flow")
        client = _required_boogie_client(evaluation_root)
    except SystemExit as error:
        return str(error)
    with tempfile.TemporaryDirectory(prefix="move-inference-prover-probe-") as temporary:
        probe = Path(temporary).resolve()
        package = probe / "package"
        (package / "sources").mkdir(parents=True)
        (package / "Move.toml").write_text(
            '[package]\nname = "probe"\nversion = "0.0.0"\n', encoding="utf-8"
        )
        (package / "sources" / "probe.move").write_text(PROBE_MODULE, encoding="utf-8")
        (probe / "home").mkdir()
        socket_path = probe / "boogie.sock"
        report = probe / "report.json"
        mounts = [(landlock, Path("/opt/bin/landlock-exec")), (client, AGENT_BOOGIE), (move_flow, move_flow), (z3, z3)]
        command = [
            str(bwrap),
            "--die-with-parent",
            "--new-session",
            "--unshare-user",
            "--unshare-pid",
            "--unshare-ipc",
            "--unshare-uts",
            "--uid",
            "0",
            "--gid",
            "0",
            "--cap-drop",
            "ALL",
            "--ro-bind",
            "/usr",
            "/usr",
            "--ro-bind",
            "/lib",
            "/lib",
            "--symlink",
            "usr/bin",
            "/bin",
            "--dev",
            "/dev",
            "--tmpfs",
            "/tmp",
            "--ro-bind",
            "/proc",
            "/proc",
        ]
        if Path("/lib64").exists():
            command.extend(("--ro-bind", "/lib64", "/lib64"))
        for directory in _missing_parent_directories([target for _, target in mounts] + [probe]):
            command.extend(("--dir", str(directory)))
        for source, target in mounts:
            command.extend(("--ro-bind", str(source), str(target)))
        command.extend(("--bind", str(probe), str(probe)))
        environment = {
            "PATH": "/usr/bin:/bin",
            "HOME": str(probe / "home"),
            "TMPDIR": "/tmp",
            "BOOGIE_EXE": str(AGENT_BOOGIE),
            "Z3_EXE": str(z3),
            "MOVE_INFERENCE_BOOGIE_PROXY": str(socket_path),
        }
        command.append("--clearenv")
        for name, value in environment.items():
            command.extend(("--setenv", name, value))
        command.extend(("--chdir", str(probe), "/opt/bin/landlock-exec"))
        for path in ("/usr", "/lib", "/opt", "/proc/self", str(move_flow), str(z3)):
            command.extend(("--ro", path))
        for path in ("/dev", "/tmp", str(probe)):
            command.extend(("--rw", path))
        command.extend((
            "--",
            "/bin/sh",
            "-c",
            f"{shlex.quote(str(move_flow))} experiment prove --package {shlex.quote(str(package))} "
            f"--target 0x42::probe::identity --timeout 10 --output {shlex.quote(str(report))}",
        ))

        async def exercise() -> tuple[int | None, str, str]:
            async with BoogieProxy(socket_path, boogie, (probe, Path("/tmp"))):
                process = await asyncio.create_subprocess_exec(
                    *command,
                    stdout=asyncio.subprocess.PIPE,
                    stderr=asyncio.subprocess.PIPE,
                )
                try:
                    stdout, stderr = await asyncio.wait_for(process.communicate(), 180)
                except TimeoutError:
                    process.kill()
                    return None, "", "prover pipeline probe timed out"
            return process.returncode, stdout.decode(errors="replace"), stderr.decode(errors="replace")

        returncode, stdout, stderr = _run_coroutine(exercise())
        proved = False
        errors: list[str] = []
        if report.is_file():
            try:
                data = json.loads(report.read_text(encoding="utf-8"))
                proved = data.get("passed") is True
                errors = [
                    line.splitlines()[0]
                    for line in data.get("diagnostics", [])
                    if isinstance(line, str) and line.startswith("error")
                ]
            except (OSError, json.JSONDecodeError) as error:
                # A report that cannot be read is a probe failure like any
                # other, and the reason belongs in the message: this probe's
                # diagnostics are the only account of why a round refused to
                # start, so swallowing it leaves a bare exit status.
                errors = [f"unreadable prover report {report}: {error}"]
    if returncode == 0 and proved:
        return None
    return "; ".join(part for part in (*errors, stderr.strip(), stdout.strip(), f"exit status {returncode}") if part)


def _run_coroutine(coroutine):
    """Run a coroutine whether or not an event loop is already running."""
    try:
        asyncio.get_running_loop()
    except RuntimeError:
        return asyncio.run(coroutine)
    import concurrent.futures

    with concurrent.futures.ThreadPoolExecutor(max_workers=1) as pool:
        return pool.submit(asyncio.run, coroutine).result()


def _preflight_result(
    passed: bool,
    detail: str,
    bwrap: Path | None,
    isolation: bool | None = None,
    prover_pipeline: bool | None = None,
) -> dict[str, object]:
    """A preflight verdict, with its two probes reported separately.

    A round needs both, so `passed` stays their conjunction. They answer
    different questions, though: `isolation` is the security property -- host
    paths hidden, another process unreadable, a read-only file not truncatable
    -- while `prover_pipeline` asks whether this host can run the solver chain
    at all. Folding them into one boolean makes a host with no Boogie report
    the sandbox policy as failing, which is untrue and invites the failure to
    be waved off as environmental.
    """
    return {
        "schema_version": 2,
        "policy_version": POLICY_VERSION,
        "passed": passed,
        "isolation": isolation,
        "prover_pipeline": prover_pipeline,
        "detail": detail,
        "bubblewrap": str(bwrap) if bwrap else None,
    }


def parse_launch(argv: list[str]) -> Launch:
    if len(argv) < 3:
        raise SystemExit("sandbox wrapper requires a controller command")
    executable = Path(argv[0]).absolute()
    if executable.resolve() != Path(sys.executable).resolve():
        raise SystemExit("sandbox child must use the wrapper's Python interpreter")
    if argv[1:3] != ["-m", "harness.controller"]:
        raise SystemExit("sandbox wrapper only launches `python -m harness.controller`")
    _reject_development_options(argv)

    parser = argparse.ArgumentParser(prog="move-inference-pilot-sandbox controller")
    parser.add_argument("--config", type=Path, required=True)
    parser.add_argument("--run", type=Path, required=True)
    parser.add_argument("--artifacts", type=Path, required=True)
    parser.add_argument("--skip-hidden-scoring", action="store_true")
    parser.add_argument("--refutation-mutants-root", type=Path)
    args = parser.parse_args(argv[3:])
    if not args.skip_hidden_scoring:
        raise SystemExit("the unscored pilot must use --skip-hidden-scoring")

    evaluation_root = Path(__file__).resolve().parent.parent
    run_manifest = args.run.resolve()
    round_dir = _round_directory(evaluation_root, run_manifest)
    if (
        round_dir is None
        or run_manifest.parent != (round_dir / "schedule" / "runs").resolve()
        or not run_manifest.is_file()
    ):
        raise SystemExit(
            "run manifest must be a round schedule under evaluation-artifacts/<round>/schedule/runs"
        )

    # The wrapper, not its caller, decides what may be mounted. A round supplies
    # its own configuration so that rounds can differ in apparatus, but every
    # path still has to belong to the same round directory.
    config = args.config.resolve()
    allowed_configs = {
        (evaluation_root / "config" / "default.json").resolve(),
        (round_dir / "config.json").resolve(),
    }
    if config not in allowed_configs:
        raise SystemExit(
            f"pilot sandbox requires the canonical config or {round_dir / 'config.json'}"
        )
    experiment_config = ExperimentConfig.load(config)
    resolved_run = RunSpec.load(run_manifest).resolve_paths(run_manifest)

    artifacts = args.artifacts.resolve()
    round_artifacts = (round_dir / "runs").resolve()
    if artifacts != round_artifacts or not artifacts.is_dir():
        raise SystemExit(f"pilot sandbox requires artifacts directory {round_artifacts}")
    if (artifacts / resolved_run.spec.run_id).exists():
        raise SystemExit(f"run artifact already exists: {resolved_run.spec.run_id}")

    _require_inside_round(
        round_dir,
        (
            ("shared_package", resolved_run.shared_package),
            ("task_patch", resolved_run.task_patch),
            ("plugin_dir", resolved_run.plugin_dir),
        ),
    )

    refutation_mutants = None
    if args.refutation_mutants_root is not None:
        refutation_mutants = args.refutation_mutants_root.resolve()
        if not refutation_mutants.is_dir():
            raise SystemExit(f"refutation mutant root is not a directory: {refutation_mutants}")
        _require_confined_mount(
            refutation_mutants, (resolved_run.plugin_dir, resolved_run.shared_package)
        )
    move_flow = _required_executable("move-flow")
    claude = _required_executable("claude", preserve_lookup_path=True)
    landlock = _required_landlock(evaluation_root)
    boogie = _required_solver("BOOGIE_EXE", "boogie")
    boogie_client = _required_boogie_client(evaluation_root)
    z3 = _required_solver("Z3_EXE", "z3")
    python_root = _python_environment_root(executable)
    return Launch(
        command=tuple(argv),
        config=config,
        run_manifest=run_manifest,
        artifacts=artifacts,
        run_id=resolved_run.spec.run_id,
        # A run that does not state its level takes the configuration's, as
        # the controller does; a raw `None` would read as "not baseline".
        feedback_level=resolved_run.spec.feedback_level or experiment_config.feedback_level,
        shared_package=resolved_run.shared_package,
        task_patch=resolved_run.task_patch,
        plugin=resolved_run.plugin_dir,
        evaluation_root=evaluation_root,
        python_root=python_root,
        move_flow=move_flow,
        claude=claude,
        boogie=boogie,
        boogie_client=boogie_client,
        z3=z3,
        landlock=landlock,
        refutation_mutants=refutation_mutants,
    )


def _required_boogie_client(evaluation_root: Path) -> Path:
    client = evaluation_root / "sandbox" / BOOGIE_PROXY_CLIENT
    if not client.is_file() or not os.access(client, os.X_OK):
        raise SystemExit(f"Boogie proxy client is missing or not executable: {client}")
    return client.resolve()


def _reject_development_options(command: list[str]) -> None:
    """Refuse the options that stand in for a real session.

    A pilot run is evidence, and an option that substitutes a scripted agent or
    skips the sandbox produces something that looks like evidence and is not.
    """
    forbidden = {"--fake-script", "--allow-unsandboxed"}
    present = sorted(forbidden.intersection(command))
    if present:
        raise SystemExit(
            f"production sandbox rejects development option: {', '.join(present)}"
        )
    if "--agent" in command:
        try:
            agent = command[command.index("--agent") + 1]
        except IndexError as error:
            raise SystemExit("--agent is missing its value") from error
        if agent != "claude":
            raise SystemExit("production sandbox permits only --agent claude")


def _require_inside_round(
    round_dir: Path, named_paths: tuple[tuple[str, Path], ...]
) -> None:
    """Reject manifest-named paths that leave the round directory.

    The manifest names paths that are mounted into the session, and the
    scheduler writes them relative to the manifest, so a relative path can climb
    out of the round. Without this check a crafted manifest could mount an
    arbitrary local tree into a session and disclose it to the model provider.
    """
    for name, path in named_paths:
        if not path.resolve().is_relative_to(round_dir):
            raise SystemExit(
                f"run manifest {name} `{path}` escapes the round directory {round_dir}"
            )


def _round_directory(evaluation_root: Path, path: Path) -> Path | None:
    """The round directory a path belongs to, or `None` if it is outside one.

    Every mounted path must live under one `evaluation-artifacts/<round>/`, so
    passing different arguments cannot direct the wrapper elsewhere.
    """
    artifacts_root = (evaluation_root / "evaluation-artifacts").resolve()
    try:
        relative = path.relative_to(artifacts_root)
    except ValueError:
        return None
    if not relative.parts:
        return None
    return artifacts_root / relative.parts[0]


def _required_landlock(evaluation_root: Path) -> Path:
    """The compiled Landlock helper that confines the agent process.

    Built rather than shipped, so it matches the kernel headers of the host it
    runs on. Absent, the run would proceed with only the outer namespace, so a
    missing binary stops the launch instead of silently weakening it.
    """
    binary = evaluation_root / "sandbox" / LANDLOCK_EXEC
    if not binary.is_file() or not os.access(binary, os.X_OK):
        raise SystemExit(
            f"{binary} is missing; build it with "
            f"`cc -O2 -o {binary} {evaluation_root / 'sandbox' / LANDLOCK_SOURCE}`"
        )
    return binary.resolve()


def _require_confined_mount(root: Path, agent_readable: tuple[Path, ...] = ()) -> None:
    """Refuse a mount that would shadow the sandbox's own filesystem.

    A bind mount is placed at its own absolute path, into a namespace that has
    already put a tmpfs at `/tmp` and granted the agent read access to the
    toolchain. A root at `/tmp`, `/opt` or any of their parents would therefore
    replace the isolated view with the host tree, which the agent can read and
    send onward.

    Containment in the evaluation directory is the whole of the check: every
    such path is either at or above a directory the sandbox establishes, and so
    outside the evaluation tree. Enumerating the sandbox's own mounts here as
    well would be a second, hand-maintained copy of `build_bwrap_command` and
    `agent_landlock_paths` -- and it had already fallen behind them.
    """
    evaluation_root = Path(__file__).resolve().parent.parent
    if not root.is_relative_to(evaluation_root):
        raise SystemExit(
            f"refutation mutant root {root} is outside {evaluation_root}; a mount "
            "is placed at its own absolute path, so a root elsewhere would shadow "
            "part of the sandbox's own filesystem rather than add to it"
        )
    # Inside the evaluation tree is necessary but not sufficient: the plugin
    # directory and the workspace are inside it too, and the agent reads both.
    # A root nested under either would be mounted where `Read` and `Glob` reach
    # it, handing over the mutations that the category-only feedback exists to
    # withhold.
    for readable in agent_readable:
        readable = Path(readable).resolve()
        if root == readable or root.is_relative_to(readable):
            raise SystemExit(
                f"refutation mutant root {root} is inside {readable}, which the "
                "agent can read; the mutants would be legible to the session "
                "that the categories are meant to keep them from"
            )

def _required_executable(name: str, preserve_lookup_path: bool = False) -> Path:
    value = shutil.which(name)
    if value is None:
        raise SystemExit(f"required executable is not on PATH: {name}")
    path = Path(value)
    return path.absolute() if preserve_lookup_path else path.resolve()


def _required_solver(environment_name: str, fallback: str) -> Path:
    """Resolve a solver, preferring an explicitly configured path.

    A configured path that is not there is an error rather than a reason to
    fall back: a round records which binaries ran, so quietly substituting a
    different solver than the operator named would misdescribe the apparatus.
    The message therefore has to distinguish a stale setting from a missing
    install, which is the difference between editing a variable and installing
    a toolchain.
    """
    configured = os.environ.get(environment_name)
    on_path = shutil.which(fallback)
    value = configured or on_path
    if not value:
        raise SystemExit(f"{environment_name} is unset and `{fallback}` is not on PATH")
    path = Path(value).resolve()
    if not path.is_file() or not os.access(path, os.X_OK):
        source = f"{environment_name}={configured}" if configured else f"`{fallback}` on PATH"
        alternative = (
            f"; `{fallback}` on PATH is {on_path}" if configured and on_path else ""
        )
        raise SystemExit(f"solver is not executable: {path} (from {source}){alternative}")
    return path


def _python_environment_root(executable: Path) -> Path | None:
    # A virtual-environment interpreter is <venv>/bin/python.  System Python
    # is already provided by the read-only /usr mount.
    if executable.is_relative_to(Path("/usr")):
        return None
    if executable.parent.name != "bin":
        raise SystemExit(f"unsupported Python interpreter layout: {executable}")
    root = executable.parent.parent
    if not (root / "pyvenv.cfg").is_file():
        raise SystemExit("non-system Python must belong to a virtual environment")
    return root


def run_isolated(launch: Launch) -> int:
    staging_parent = launch.artifacts / ".sandbox-staging"
    staging_parent.mkdir(mode=0o700, parents=True, exist_ok=True)
    staging = Path(
        tempfile.mkdtemp(prefix=f"{launch.run_id}.", dir=staging_parent)
    ).resolve()
    (staging / ".sandbox-home").mkdir(mode=0o700)
    published = launch.artifacts / launch.run_id
    # A dispatch abort terminates this process, and the default disposition for
    # SIGTERM and SIGINT ends it without unwinding -- so the `finally` below
    # would not run, and the staging tree would survive holding whatever the
    # agent wrote into its workspace, including a credential it read from its
    # own `/proc/self`. Raising from the handler turns the signal into an
    # ordinary unwind so the cleanup happens.
    with _cleanup_on_termination():
        try:
            command = build_bwrap_command(launch, staging)
            process = subprocess.run(command, check=False)
            staged_run = staging / launch.run_id
            if staged_run.exists():
                if published.exists():
                    raise RuntimeError(f"refusing to overwrite run artifact: {published}")
                staged_run.rename(published)
            return process.returncode
        finally:
            shutil.rmtree(staging, ignore_errors=True)
            try:
                staging_parent.rmdir()
            except OSError:
                pass


@contextlib.contextmanager
def _cleanup_on_termination():
    """Make a terminating signal unwind rather than kill outright.

    The dispatcher stops an abandoned cell with SIGTERM before escalating, and
    the default disposition ends this process without running any `finally`.
    Handling the signal by raising converts it into an ordinary unwind, so the
    staging tree -- which holds the agent's workspace verbatim -- is removed.
    SIGKILL cannot be caught; the grace period before it is what makes this
    worth having.
    """

    def terminate(number, _frame):
        raise SystemExit(128 + number)

    previous = {}
    for number in (signal.SIGTERM, signal.SIGINT, signal.SIGHUP):
        with contextlib.suppress(ValueError, OSError):
            previous[number] = signal.signal(number, terminate)
    try:
        yield
    finally:
        for number, handler in previous.items():
            with contextlib.suppress(ValueError, OSError):
                signal.signal(number, handler)


def build_bwrap_command(launch: Launch, staging: Path) -> list[str]:
    bwrap = _required_executable("bwrap")
    mounts: list[tuple[Path, Path, bool]] = []

    def mount(source: Path, target: Path | None = None, writable: bool = False) -> None:
        mounts.append((source.resolve(), (target or source).absolute(), writable))

    mount(launch.evaluation_root / "harness")
    mount(launch.evaluation_root / "prompts")
    mount(launch.config)
    mount(launch.run_manifest)
    mount(launch.shared_package)
    mount(launch.task_patch)
    mount(launch.plugin)
    # KNOWN LEAK RISK: refutation needs these inside the namespace the agent
    # shares, so only its Landlock ruleset withholds them -- one mechanism where
    # scoring material had two. Never mount the set the round is scored on;
    # `harness.controller` refuses if the two roots are equal.
    if launch.refutation_mutants is not None:
        mount(launch.refutation_mutants)
    mount(staging, launch.artifacts, writable=True)
    if launch.python_root is not None:
        mount(launch.python_root)
    mount(launch.move_flow)
    # The agent runs through a wrapper that applies the Landlock ruleset and
    # then execs the real binary, so `claude` on the path is the wrapper.
    mount(launch.landlock, Path("/opt/bin/landlock-exec"))
    mount(_write_agent_wrapper(launch, staging), Path("/opt/bin/claude"))
    mount(launch.claude.resolve(), Path("/opt/bin/claude-real"))
    mount(launch.boogie)
    mount(launch.boogie_client, AGENT_BOOGIE)
    mount(launch.z3)

    command = [
        str(bwrap),
        "--die-with-parent",
        "--new-session",
        "--unshare-user",
        "--unshare-pid",
        "--unshare-ipc",
        "--unshare-uts",
        "--uid",
        "0",
        "--gid",
        "0",
        "--cap-drop",
        "ALL",
        "--ro-bind",
        "/usr",
        "/usr",
        "--ro-bind",
        "/lib",
        "/lib",
        "--symlink",
        "usr/bin",
        "/bin",
        "--dev",
        "/dev",
        "--tmpfs",
        "/tmp",
        # Boogie's CoreCLR reads process memory information at startup and
        # aborts without this. The agent sees only what Landlock admits.
        "--ro-bind",
        "/proc",
        "/proc",
    ]
    if Path("/lib64").exists():
        command.extend(("--ro-bind", "/lib64", "/lib64"))
    if Path("/etc/ssl").is_dir():
        command.extend(("--ro-bind", "/etc/ssl", "/etc/ssl"))
    for path in RESOLVER_INPUTS:
        if path.is_file():
            command.extend(("--ro-bind", str(path), str(path)))

    targets = [target for _, target, _ in mounts]
    targets.extend(path for path in RESOLVER_INPUTS if path.is_file())
    targets.append(launch.evaluation_root)
    for directory in _missing_parent_directories(targets):
        command.extend(("--dir", str(directory)))
    for source, target, writable in mounts:
        command.extend(
            ("--bind" if writable else "--ro-bind", str(source), str(target))
        )

    executable_dirs = _unique(
        [
            str(Path(launch.command[0]).parent),
            "/opt/bin",
            str(launch.move_flow.parent),
            "/usr/bin",
            "/bin",
        ]
    )
    environment = {
        "PATH": ":".join(executable_dirs),
        "PYTHONPATH": str(launch.evaluation_root),
        "HOME": str(launch.artifacts / ".sandbox-home"),
        "TMPDIR": "/tmp",
        "LANG": "C.UTF-8",
        "LC_ALL": "C.UTF-8",
        "BOOGIE_EXE": str(launch.boogie),
        "Z3_EXE": str(launch.z3),
        # The controller serves Boogie to the agent over this socket and points
        # the agent's prover at the client; the executable stays its own.
        "MOVE_INFERENCE_BOOGIE_PROXY": str(BOOGIE_PROXY_SOCKET),
        "MOVE_INFERENCE_BOOGIE_REAL": str(launch.boogie),
        "MOVE_INFERENCE_BOOGIE_AGENT": str(AGENT_BOOGIE),
        # Claude Code launches itself by this path, so the wrapper has to be
        # what it finds rather than only what is first on PATH.
        "CLAUDE_CODE_EXECUTABLE": "/opt/bin/claude",
        "CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC": "1",
        # This capability marker is deliberately created by the wrapper after
        # --clearenv.  The parent launcher removes any host-provided value, so
        # only a controller running inside this policy can pass the real-run
        # guard.
        "MOVE_INFERENCE_EVAL_SANDBOXED": "1",
    }
    for name in (
        "ANTHROPIC_AUTH_TOKEN",
        "ANTHROPIC_API_KEY",
        "ANTHROPIC_BASE_URL",
        "SSL_CERT_FILE",
        "SSL_CERT_DIR",
    ):
        if value := os.environ.get(name):
            environment[name] = value
    command.append("--clearenv")
    for name, value in environment.items():
        command.extend(("--setenv", name, value))
    command.extend(("--chdir", str(launch.evaluation_root), "--"))
    command.extend(launch.command)
    return command


def agent_landlock_paths(launch: Launch) -> tuple[list[Path], list[Path]]:
    """What the agent process may read and write, inside the sandbox.

    Narrower than the sandbox itself: the agent edits its workspace and reads
    its plugin, `move-flow`, Z3 and the Boogie proxy client, but the harness,
    the prompts, the task patch, the pristine package and the Boogie executable
    stay out of reach even though the controller has them mounted.
    """
    readable = [
        # The loader and the shared libraries live outside `/usr` here, and
        # without them every exec fails before the program starts.
        *SYSTEM_READ_ONLY,
        *( [Path("/lib64")] if Path("/lib64").exists() else [] ),
        Path("/opt"),
        *(path for path in AGENT_PROC_PATHS if path.exists()),
        *(path for path in RESOLVER_INPUTS if path.is_file()),
        launch.plugin,
        launch.move_flow,
        # Z3 but not Boogie: `move-flow` execs Z3 itself for its version check
        # and Z3 is an ordinary binary with no `/proc/self` dependency, while
        # Boogie reaches the agent only through the proxy client under `/opt`.
        launch.z3,
    ]
    if Path("/etc/ssl").is_dir():
        readable.append(Path("/etc/ssl"))
    run_dir = launch.artifacts / launch.run_id
    readable.extend(
        [
            # The MCP server runs in this domain. It compares the workspace
            # against the pristine baseline, reads its runtime configuration,
            # and loads the plugin: all read-only to it.
            run_dir / "baseline",
            run_dir / "mcp.runtime.json",
            run_dir / "plugin",
        ]
    )
    if launch.feedback_level != "baseline":
        # The task's criteria are the acceptance intervention. The file is
        # written at every level because the judge reads it after the run, but
        # a baseline cell's server is not given it, and neither is the agent.
        readable.append(run_dir / "candidate-check.json")
    writable = [
        Path("/dev"),
        Path("/tmp"),
        launch.artifacts / ".sandbox-home",
        # The agent edits only its workspace. The rest of the run directory --
        # the pristine baseline, `run.json`, the judge's results -- belongs to
        # the controller, and an agent that could write it could forge the
        # record of its own run. The telemetry file is the one exception: the
        # MCP server that writes it inherits this domain, so the controller
        # creates the file first and the rule names that file alone.
        run_dir / "workspace",
        run_dir / "flow-events.jsonl",
    ]
    return readable, writable


def _write_agent_wrapper(launch: Launch, staging: Path) -> Path:
    """Generate the wrapper that Landlocks the agent and execs the real binary.

    Generated rather than checked in so the ruleset is built from the same
    `Launch` the mounts are, and cannot describe a different set of paths.
    """
    readable, writable = agent_landlock_paths(launch)
    arguments = []
    for path in readable:
        arguments.extend(("--ro", str(path)))
    for path in writable:
        arguments.extend(("--rw", str(path)))
    arguments.extend(("--", "/opt/bin/claude-real"))
    wrapper = staging / "claude-landlocked"
    wrapper.write_text(
        "#!/bin/sh\nset -eu\nexec /opt/bin/landlock-exec {} \"$@\"\n".format(
            " ".join(shlex.quote(argument) for argument in arguments)
        ),
        encoding="utf-8",
    )
    wrapper.chmod(0o755)
    return wrapper


def _missing_parent_directories(paths: Iterable[Path]) -> list[Path]:
    directories: set[Path] = set()
    protected = {Path("/usr"), Path("/lib"), Path("/lib64"), Path("/etc/ssl")}
    for path in paths:
        parent = path if path.suffix == "" and path.is_dir() else path.parent
        while parent != Path("/"):
            if not any(parent == root or parent.is_relative_to(root) for root in protected):
                directories.add(parent)
            parent = parent.parent
    return sorted(directories, key=lambda value: (len(value.parts), str(value)))


def _unique(values: Iterable[str]) -> list[str]:
    return list(dict.fromkeys(values))


if __name__ == "__main__":
    main()
