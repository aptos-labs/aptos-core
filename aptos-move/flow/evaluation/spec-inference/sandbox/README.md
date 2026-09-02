# The evaluation sandbox

Every real model session runs inside this sandbox. This document states what it
is for, what it actually enforces, and — as precisely as the rest — what it does
not. A reviewer judging a claimed sandbox weakness should be able to decide from
this page whether the claim is inside the threat model or outside it.

The implementation is `harness/pilot_sandbox.py` (mounts, environment, and the
generated wrapper), `sandbox/landlock_exec.c` (the inner ruleset), and
`harness/boogie_proxy.py` with `sandbox/boogie-proxy-client.py` (the solver
path). `POLICY_VERSION` in `pilot_sandbox.py` is bumped whenever the policy
changes; preflight records it, and the audit compares it.

## What it is for

The agent's output is untrusted, and so is anything it reads. The sandbox exists
to make three properties hold regardless of what the model does:

1. **Hidden material stays hidden.** Mutants and reference specifications must
   never enter a session. A contract that verifies is only evidence if it was
   written without seeing the answer.
2. **The apparatus cannot be edited from inside.** Harness, prompts, task patch,
   pristine baseline, and the round's own configuration are not writable by the
   agent, so a session cannot rewrite the thing that scores it.
3. **A run cannot forge its own record.** The agent writes only its workspace;
   telemetry and artifacts are the controller's.

These are integrity properties about *the experiment*. They are not a claim that
a hostile model can be safely run on a machine with secrets on it — see
[Non-goals](#non-goals).

## Two independent layers

The outer layer is a bubblewrap namespace; the inner layer is a Landlock ruleset
applied by a wrapper that then execs the real binary. They are separate on
purpose: the sandbox mounts what the *controller* needs (it runs Boogie, reads
the task patch, writes telemetry), while the agent is confined to a strict subset
of that.

```
bwrap namespace              ← controller's view: solver, patch, prompts, artifacts
  └── landlock-exec ruleset  ← agent's view: workspace, plugin, move-flow, Z3
        └── claude
```

The wrapper is generated per launch from the same `Launch` value that builds the
mounts (`_write_agent_wrapper`), so the ruleset cannot drift from the mount set
and describe a different world than the one that exists.

### Outer namespace

`--unshare-user --unshare-pid --unshare-ipc --unshare-uts`, `--cap-drop ALL`,
uid/gid 0 inside only, `--die-with-parent`, `--new-session`, and `--clearenv`
followed by an explicit variable list. `/usr` and `/lib` are read-only, `/tmp` is
a tmpfs, `/dev` is minimal.

`/proc` is mounted read-only, which is a deliberate concession: Boogie's
self-contained CoreCLR reads `/proc/self` at startup and aborts with
`HRESULT: 0x8007000E` without it. The agent does not get all of `/proc` — see
below.

### Inner ruleset

| agent may read | agent may write |
|---|---|
| `/usr`, `/lib`, `/lib64`, `/opt` | its workspace, `artifacts/<run_id>/workspace` |
| `/proc/self`, `/proc/stat`, `/proc/sys/vm` | the telemetry file, `artifacts/<run_id>/flow-events.jsonl` |
| its rendered plugin | `artifacts/.sandbox-home` |
| the pristine baseline, the task's criteria, the runtime MCP configuration | `/tmp`, `/dev` |
| `move-flow`, Z3, the Boogie proxy client | |
| TLS roots and resolver inputs | |

Truncation is governed too, from Landlock ABI 3 onwards: it is a right of its
own, and left out of the policy a file the ruleset keeps read-only could still
be emptied through `O_TRUNC` or `ftruncate`. Preflight probes that a read-only
file cannot be truncated.

The rest of the run directory — `run.json`, the judge's results, the event
logs — is the controller's. An agent that could write it could forge the record
of its own run, so it cannot. The telemetry file is writable because the MCP
server that writes it inherits the agent's domain; the controller creates it
before launch so that a rule can name that one file.

Deliberately **absent** from the agent's ruleset, though the controller has them
mounted: the harness, the prompts, the task patch, the pristine baseline package,
and the Boogie executable.

`/proc` is the subtle one. `/proc/self` under Landlock resolves to the *wrapper's*
own PID and to no child's, so the agent cannot read another process — preflight
asserts exactly this by reading `/proc/self/status` and requiring `/proc/1/status`
to fail.

## Why the solver goes through a proxy

Because `/proc/self` pins to the wrapper PID, an agent-spawned Boogie would
resolve `/proc/self` to a PID its own domain cannot read, and CoreCLR would abort
before running. Rather than widen the ruleset, the controller serves the real
Boogie over a Unix socket and the agent's `BOOGIE_EXE` is a client script.

This is not only a workaround: it means the agent never holds the solver
executable, and every solver invocation passes through a controller-side
validation of the working directory.

Preflight proves this end to end by running a one-line Move package through the
whole agent-side chain — `move-flow` → proxy client → controller's Boogie → Z3 —
as the grandchild it really is at runtime. An earlier revision only exec'd Boogie
directly from the wrapper, which works and therefore proved nothing; several
rounds ran with a prover that could not start.

## Tools

The agent's capability surface is the union of its builtin tools and the MCP
server's inventory. `config/default.json` allows `Read`, `Glob`, `Grep`, `Edit`,
`Write`, and the task tools, and denies `Bash`, `WebSearch`, `WebFetch`, `Agent`,
and `Task`.

Denying `Bash` and `WebFetch` matters more than it looks: with them gone, MCP
tools are the *only* channel out of the process. That makes the MCP inventory a
security boundary, not just an ergonomics choice — any tool that takes a URL is
an egress channel, and is treated as one.

For that reason `move_replay_transaction` is not served in an evaluation
session: it reaches an arbitrary REST endpoint and sends a caller-supplied key
as a bearer token, and it has no use while specifying a package. The tool
inventory is asserted by a test, not left to review.

## Network

Networking is **not** isolated: there is no `--unshare-net`. The session has to
reach the model endpoint, and the round would not run without it.

Everything else in the design assumes this. Confidentiality of anything the agent
can read therefore rests on the agent having no way to *send* — which is why the
tool inventory above is part of the sandbox and not separate from it.

## Credentials

`with-glm-env.sh` resolves the provider key and exports it as
`ANTHROPIC_AUTH_TOKEN`; it never prints the value or writes it to an artifact,
and it unsets the source variables. The launcher forwards
`ANTHROPIC_AUTH_TOKEN`, `ANTHROPIC_API_KEY`, `ANTHROPIC_BASE_URL`, and the TLS
variables into the namespace after `--clearenv`.

The agent process needs the token — it is the thing calling the API — so the
token is in the environment of a process whose `/proc/self` the agent can read.
**An agent that wants its own credential can obtain it.** The sandbox does not
try to prevent that; it prevents the credential from *leaving*, by ensuring there
is no egress channel (see Network and Tools above).

The controller also sweeps the run's artifacts for the credential's exact
bytes once the run is over, so an agent that merely echoes it does not put it
into a transcript that is later quoted. The round's `launch-report.json` sits
outside that per-run tree and quotes the controller's own output, which can in
turn quote agent-written source — a compiler diagnostic echoes the line it
rejected — so the dispatcher redacts what it records there. `harness/
credentials.py` is the single definition of the credential variables and the
replacement, and every writer that can receive agent-influenced bytes goes
through it. That is best-effort by construction:
an agent that encodes or splits the value defeats a byte sweep, and no sweep
can be complete, so a session's artifacts are untrusted data. The design that
closes this is an authentication proxy on the controller's side, so that the
agent never holds the credential at all — the same shape as the Boogie proxy.

`MOVE_INFERENCE_EVAL_SANDBOXED=1` is created by the wrapper after `--clearenv`,
and the parent launcher strips any host-provided value, so only a controller
running inside the policy can pass the real-run guard.

## What preflight proves

Preflight runs before a round and records, in `preflight.json`: the policy
version, the `bwrap` and `landlock-exec` binaries with their SHA-256, that a
host-only sentinel path is invisible inside the namespace, that another process
is unreadable under the agent ruleset, that a read-only file cannot be
truncated, and that the full prover pipeline runs as an agent grandchild. A
round whose preflight did not pass is not a round.

The last of those is reported separately, as `prover_pipeline`, from the
`isolation` probe that covers the rest. `passed` is still their conjunction and
the round gate is unchanged, but they answer different questions: isolation is
the security property, while the pipeline probe asks whether this host has a
working solver chain at all. A host with a stale `BOOGIE_EXE` would otherwise
report the sandbox policy as failing, which is untrue and invites a real
regression to be waved off as environmental. `_required_solver` refuses a
configured path that is not there rather than falling back to `PATH` — a round
records which binaries ran, so substituting a solver the operator did not name
would misdescribe the apparatus — and its message names the stale variable and
what is on `PATH`, so the two cases are distinguishable.

Nothing in CI runs these tests: `tests/test_pilot_sandbox.py` is the only
automated exercise of the real `landlock-exec` binary, and it runs when someone
runs it. Treat a failure there as a finding, not as noise.

## Path validation

The wrapper, not its caller, decides what may be mounted. The run manifest must
live under `evaluation-artifacts/<round>/schedule/runs`, the config must be the
canonical one or the round's own, and the artifacts directory must be the round's
`runs/`.

The manifest additionally names three paths that are mounted into the session —
`shared_package`, `task_patch`, and `plugin_dir` — and the scheduler writes them
*relative to the manifest*, so a relative path can climb out of the round. Each
is therefore checked to resolve inside the round directory before it is mounted.
Without that check a crafted manifest could mount an arbitrary local tree into a
session and disclose it to the model provider.

## Non-goals

Stated plainly, so that a finding can be classified rather than argued:

- **Not a defense against a model with an egress channel.** The design keeps the
  channel closed. If a tool that reaches the network is added to the inventory,
  the confidentiality argument above is void — the credential and the workspace
  both become reachable. Adding such a tool is a policy change, not a feature,
  and the inventory test in the flow crate is what makes that deliberate.
- **Not a multi-tenant or hostile-code sandbox.** It confines one cooperating
  process on a researcher's own machine. It is not hardened against kernel
  escapes, side channels, or resource exhaustion; a session can burn CPU.
- **Not a guarantee about the corpus.** The sandbox assumes corpus content is
  authored by the study, not attacker-supplied. Prompt injection through a
  malicious Move source is out of scope, and would be a corpus-integrity problem
  before it is a sandbox one.
- **Not a guarantee of telemetry integrity.** The Flow MCP server runs as a
  child of the agent process and so inherits the agent's Landlock domain: it can
  write only where the agent can write, which is why `flow-events.jsonl` lives in
  the run directory. An agent with `Write` could therefore rewrite it, including
  the `session_start` records the controller reads to detect a restarted MCP
  supervisor. Filesystem rules cannot separate that writer from the agent;
  closing this needs a controller-side telemetry channel, on the pattern the
  Boogie proxy already uses. Until then, telemetry is evidence the agent could
  have altered, and is not treated as proof against a deliberately hostile one.
- **Not a substitute for the audit.** Blindness to hidden material is enforced by
  mounts; that the *right* material was hidden is checked afterwards by
  `pilot_audit.py` with `--forbidden-path`.
