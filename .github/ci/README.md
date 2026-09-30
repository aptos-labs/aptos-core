# Python CI contract tests

The tests use `unittest` and Hypothesis to generate bounded inputs and simplify
failures. Coverage includes all Python tests added or changed by the CI/CD
hardening: CI helpers, benchmark reports, and Forge, faucet, and E2E image
controls. Test dependencies are separate from production dependencies.

## Install and run

Use CPython 3.12. The full suite supports Linux x86_64 and macOS x86_64/arm64.
The Forge overlay retains psutil 5.9.8, which has no Linux arm64 wheel. The
central suite's smaller dependency file also supports Linux arm64. The root
`.python-version` still selects Python 3.9 for other tools.

From the repository root:

```sh
python3.12 -m venv /tmp/aptos-ci-tests-venv
source /tmp/aptos-ci-tests-venv/bin/activate
python -m pip install --require-hashes --only-binary=:all: -r .github/ci/requirements-forge-test.txt
python .github/ci/run_python_tests.py
```

The runner streams each group's output and fails if any group fails. It uses
the current interpreter and a separate process for each group. This prevents
the two benchmark test modules and the two `common` modules from colliding.

| `--suite` value | Tests |
| --- | --- |
| `central` | All eleven `.github/ci/tests/test_*.py` modules |
| `micro-report` | MonoMove micro-benchmark report producer |
| `e2e-report` | MonoMove E2E report producer |
| `forge` | Complete Forge unit suite, including protected images |
| `faucet-images` | Faucet tools image references |
| `e2e-images` | E2E tools image references |
| `offline-images` | E2E and faucet offline Docker command contracts |

For example, run one group with `python .github/ci/run_python_tests.py --suite forge`.
The central tests also support normal discovery:

```sh
python -m unittest discover -s .github/ci/tests -t .github/ci
```

Install `requirements-test.txt` instead of the Forge overlay when running only
central contracts or benchmark reports. Missing Hypothesis fails discovery;
properties are never silently skipped. Production helpers do not import it.

## Profiles and failure replay

`HYPOTHESIS_PROFILE=ci` is the default. It runs up to 100 deterministic examples
per property. To search more inputs locally:

```sh
HYPOTHESIS_PROFILE=explore python .github/ci/run_python_tests.py
HYPOTHESIS_PROFILE=explore python .github/ci/run_python_tests.py --suite central
```

The exploratory profile runs up to 1,000 randomized examples per property.
Both profiles disable the example database and per-example deadline. Health
checks remain enabled. An unknown profile name fails before the unified runner
starts a group and also fails ordinary discovery.

Generated failures print a simplified input and a `@reproduce_failure` decorator. Replay
with the same Hypothesis version: temporarily add that decorator to the failing
test and rerun its discovery command or suite group. For inherited tools-image
properties, put the decorator on the shared method in `harness_support.py` and
select the affected faucet or E2E group. Replace the temporary decorator with a
named regression or `@example` containing the concrete input when keeping the
case permanently. The opaque replay blob is specific to a Hypothesis version.
An explicit `@example` failure prints its fixed input. Rerun that example directly;
it does not need a generated replay blob.

## Security requirements and test reduction

| Requirement | Retained test coverage |
| --- | --- |
| Valid REST inputs, bounded responses, redacted failures, and complete pagination | Generated parsing/response/page properties; real transport and launcher regressions |
| Latest eligible label actor has current write/admin permission | Independent event models, explicit permission categories, batch/cache properties, and timestamp/bot regressions |
| Downstream dispatch stays bound to one correlation and run ID | Payload and scripted polling properties; exact request/order/timeout regressions |
| Manifest relationships and approvals select exact images/workloads | Independent generated graph models, canonical plans, status aggregation, and exhaustive approval subsets |
| Protected manifests bind every identity and use a trusted controller | All variants and identity fields; attempt/digest/schema properties and controller regressions |
| Trusted run origin and well-formed identity | Origin properties and explicit workflow mismatch/display-name regressions |
| One exact PR match, independent of candidate order | Binding properties; explicit deleted-repository, stale-head, request-order, URL, and pagination regressions |
| Reject malformed PR candidate metadata | Properties assert the specific validation error, so another rejection guard cannot hide a missing check |
| Valid build/E2E archives and fixed command/output contracts | Independent archive fixtures, positive properties, and named command regressions |
| Same/earlier attempts accepted; future attempts rejected | Attempt property and named rerun regression |
| Every bound identity field matches | Identity property explicitly visits each field except the separately tested attempt |
| Reject invalid digest/file/metadata/tar structure | Rejection property explicitly visits every listed category in both archive modes |
| Validate every archive before publishing/loading or writing outputs | Rejections corrupt the last archive and assert zero subprocess calls and unchanged environment output |
| Reports contain only bound, typed, bounded, safe data | Schema/renderer properties, every finite rejection category, numeric boundaries, and golden comments |
| Exactly one valid producer job/step/artifact is consumed | Selection/permutation properties, completed execution metadata, bounded ZIP structures, and redirect regressions |
| Benchmark rows preserve trusted identity and numeric meaning | Independent adapter calculations, actual JSON-line conversion, nullable rows, and staged-import regressions |
| Approved harness images use digests; offline commands do not pull | Shared actual-helper contracts, separate controller/pod properties, and mocked command assertions |

Finite rejection categories run explicitly. Random selection does not decide
which security rules get tested. Ordinary examples use small graphs, page
sequences, archives, and up to twenty timeline events or report rows. Accepted
limits and adjacent invalid values have explicit examples. Expected decisions,
tags, file names, digests, and report calculations follow test-side requirements.

Generated examples use fresh fixtures, environment patches, temporary files,
transports, clocks, and subprocess mocks. They do not contact external services
or run Docker, Skopeo, or registry commands. Existing real loopback transport,
launcher, file-output, and staged-import tests remain explicit integration
checks. SDK stubs only satisfy unrelated imports while image tests load the
actual helper modules with isolated names.

Named regressions reject numeric-equivalent boolean/float API identities,
changed completion run IDs, non-string conclusions/statuses, hostless URLs,
empty user information, and malformed URL ports. These are response-validation
controls; synthetic malformed responses alone do not establish attacker impact.

Remove repeated cases only after a one-time mutation audit shows that retained
tests detect the corresponding weakened controls. An assertion failure counts
as detection; import errors, syntax errors, unexpected exceptions, and timeouts
do not. Tests assert the intended exception type. Keep known security regressions
explicit. Mutation checks are migration evidence and do not run in recurring CI.

Judge reduction by retained requirements and mutation detection. Record test
count, code size, and runtime; these can increase while repeated setup decreases.
Production defects block affected removals until repaired and verified.
