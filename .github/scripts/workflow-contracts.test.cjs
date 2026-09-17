const assert = require('node:assert/strict');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const { spawnSync } = require('node:child_process');
const { test } = require('node:test');

const root = path.resolve(__dirname, '../..');

// Execute the checked-in shell bodies rather than copies of their conditions.
function stepRun(file, name) {
  const source = fs.readFileSync(path.join(root, file), 'utf8');
  const step = source.split(`name: ${name}\n`);
  assert.equal(step.length, 2, `Expected one step named ${name}`);
  const match = step[1].match(/^(\s*)run: \|\n([\s\S]*)/m);
  assert.ok(match, `No shell body for ${name}`);
  const indentation = match[1].length + 2;
  const lines = [];
  for (const line of match[2].split('\n')) {
    if (line.trim() && line.search(/\S/) < indentation) break;
    lines.push(line.slice(indentation));
  }
  return lines.join('\n');
}

function namedStep(file, name) {
  const source = fs.readFileSync(path.join(root, file), 'utf8').split('\n');
  const start = source.findIndex(line => line.trim() === `- name: ${name}`);
  assert.notEqual(start, -1, `No step named ${name}`);
  const indentation = source[start].search(/\S/);
  const end = source.findIndex((line, index) => index > start && line.trim() && line.search(/\S/) <= indentation);
  return source.slice(start, end === -1 ? undefined : end).join('\n');
}

function shell(script, env, cwd = root) {
  return spawnSync('bash', ['-e', '-o', 'pipefail', '-c', script], {
    cwd, env: { ...process.env, ...env }, encoding: 'utf8',
  });
}

test('CLI assertion tests tolerate older sources but propagate existing test failures', () => {
  const temporary = fs.mkdtempSync(path.join(os.tmpdir(), 'ci-cli-assertions-'));
  try {
    const script = stepRun('.github/workflows/cli-e2e-tests.yaml', 'Test E2E assertions');
    const trace = path.join(temporary, 'trace');
    fs.writeFileSync(path.join(temporary, 'poetry'), '#!/bin/sh\nprintf "%s\\n" "$*" >> "$PROBE_TRACE"\nexit "$PROBE_EXIT"\n', { mode: 0o700 });
    const env = { PATH: `${temporary}:${process.env.PATH}`, PROBE_TRACE: trace, PROBE_EXIT: '23' };
    const legacy = shell(script, env, temporary);
    assert.equal(legacy.status, 0, legacy.stderr);
    assert.ok(!fs.existsSync(trace), 'Older sources must not invoke unittest discovery');
    fs.mkdirSync(path.join(temporary, 'tests'));
    for (const code of ['0', '23']) {
      fs.writeFileSync(trace, '');
      const result = shell(script, { ...env, PROBE_EXIT: code }, temporary);
      assert.equal(result.status, Number(code), result.stderr);
      assert.equal(fs.readFileSync(trace, 'utf8').trim(), 'run python -m unittest discover -s tests');
    }
    assert.ok(!namedStep('.github/workflows/cli-e2e-tests.yaml', 'Test E2E assertions').includes('continue-on-error: true'));
  } finally {
    fs.rmSync(temporary, { recursive: true });
  }
});

test('measurement collection and upload are best-effort, even after a test failure', () => {
  const file = '.github/actions/ci-metrics-report/action.yaml';
  for (const name of ['Collect measurements', 'Upload measurements']) {
    const step = namedStep(file, name);
    assert.match(step, /^\s+continue-on-error: true$/m);
    assert.match(step, /^\s+if: always\(\)$/m);
  }
  const temporary = fs.mkdtempSync(path.join(os.tmpdir(), 'ci-metrics-failure-'));
  try {
    fs.mkdirSync(path.join(temporary, 'ci-metrics'));
    const phases = path.join(temporary, 'ci-metrics', 'phases.jsonl');
    fs.writeFileSync(phases, '{"phase":"tests","exit_code":0}\n');
    const result = shell(`git() { :; }\nrustc() { :; }\ncargo() { return 23; }\n${stepRun(file, 'Collect measurements')}`, {}, temporary);
    assert.equal(result.status, 23, 'A missing nextest must be tolerated by the action step');
    assert.equal(fs.readFileSync(phases, 'utf8'), '{"phase":"tests","exit_code":0}\n');
    assert.match(namedStep(file, 'Upload measurements'), /path: ci-metrics\//);
  } finally {
    fs.rmSync(temporary, { recursive: true });
  }
  for (const workflow of ['cli-e2e-tests', 'node-api-compatibility-tests']) {
    for (const name of ['Sample host memory', 'Record runner resources and image digests']) {
      assert.match(namedStep(`.github/workflows/${workflow}.yaml`, name), /^\s+continue-on-error: true$/m);
    }
  }
});

test('test artifacts and benchmark inventory verification remain mandatory', () => {
  for (const [file, names] of [
    ['.github/workflows/targeted-unit-tests.yaml', ['Save the test archive for the shards', 'Restore the test archive']],
    ['.github/workflows/smoke-tests.yaml', ['Upload aptos-node binary', 'Upload smoke test archive and helper binaries']],
  ]) {
    for (const name of names) {
      assert.ok(!namedStep(file, name).includes('continue-on-error: true'));
    }
  }
  const file = '.github/workflows/ci-benchmark.yaml';
  const name = 'Require identical inventories and complete disjoint partitions';
  assert.ok(!namedStep(file, name).includes('continue-on-error: true'));
  const temporary = fs.mkdtempSync(path.join(os.tmpdir(), 'ci-inventory-missing-'));
  try {
    const result = shell(stepRun(file, name), { PYTHONPATH: path.join(root, '.github/scripts'), PYTHONDONTWRITEBYTECODE: '1' }, temporary);
    assert.notEqual(result.status, 0, 'A benchmark with missing measurement artifacts must fail validation');
    assert.match(result.stderr, /ValueError: max\(\) (?:arg is an empty sequence|iterable argument is empty)/);
  } finally {
    fs.rmSync(temporary, { recursive: true });
  }
});

test('artifact consumers reject empty, invalid and multiple producer IDs', () => {
  for (const [file, name, fields] of [
    ['.github/workflows/targeted-unit-tests.yaml', 'Require the producer artifact ID', ['TEST_ARTIFACT_ID']],
    ['.github/workflows/smoke-tests.yaml', 'Require the producer artifact IDs', ['NODE_ARTIFACT_ID', 'TEST_ARTIFACT_ID']],
  ]) {
    const script = stepRun(file, name);
    const valid = Object.fromEntries(fields.map(field => [field, '10126978793']));
    assert.equal(shell(script, valid).status, 0);
    for (const field of fields) {
      for (const invalid of ['', '0', '-1', 'abc', '123,456']) {
        assert.notEqual(shell(script, { ...valid, [field]: invalid }).status, 0, `${name}: ${field}=${invalid}; ${script}`);
      }
    }
  }
});

test('benchmark verifier accepts inline and sharded runs and rejects incomplete coverage', () => {
  const temporary = fs.mkdtempSync(path.join(os.tmpdir(), 'ci-inventory-coverage-'));
  try {
    const script = stepRun('.github/workflows/ci-benchmark.yaml', 'Require identical inventories and complete disjoint partitions');
    const env = { PYTHONPATH: path.join(root, '.github/scripts'), PYTHONDONTWRITEBYTECODE: '1' };
    function record(name, tests, includeIgnored = true) {
      const directory = path.join(temporary, 'metrics', `ci-metrics-${name}-1`);
      fs.mkdirSync(directory, { recursive: true });
      const listing = {
        'test-count': tests.length + Number(includeIgnored),
        'rust-suites': { binary: { testcases: {
          ...Object.fromEntries(tests.map(name => [name, {
            ignored: false, 'filter-match': { status: 'matches' },
          }])),
          ...(includeIgnored ? { 'ignored-test': {
            ignored: true, 'filter-match': { status: 'mismatch', reason: 'ignored' },
          } } : {}),
        } } },
      };
      fs.writeFileSync(path.join(directory, 'inventory.json'), JSON.stringify(listing));
    }
    for (const count of [9, 3001]) {
      const tests = Array.from({ length: count }, (_, index) => `test-${index}`);
      for (const sample of [1, 2, 3]) {
        record(`baseline-${sample}`, tests);
        record(`candidate-${sample}-build`, tests);
        if (count > 3000) {
          for (let partition = 1; partition <= 8; partition++) {
            record(`candidate-${sample}-shard-${partition}`, tests.filter((_, index) => index % 8 === partition - 1));
          }
        }
      }
      const result = shell(script, env, temporary);
      assert.equal(result.status, 0, result.stderr);
      assert.equal(result.stdout.split('exact inventory and partition match').length - 1, 3);
      record('candidate-2-build', tests, false);
      const missingIgnored = shell(script, env, temporary);
      assert.notEqual(missingIgnored.status, 0, 'A missing ignored test must fail the full inventory comparison');
      assert.match(missingIgnored.stderr, /Partition mismatch/);
      record('candidate-2-build', tests);
    }
    record('candidate-3-shard-8', []);
    const incomplete = shell(script, env, temporary);
    assert.notEqual(incomplete.status, 0);
    assert.match(incomplete.stderr, /Partition mismatch/);
  } finally {
    fs.rmSync(temporary, { recursive: true });
  }
});

test('required checks reject failures, cancellations, missing results and unexpected skips', () => {
  for (const [file, name] of [
    ['.github/workflows/lint-test.yaml', 'Check targeted unit test results'],
    ['.github/workflows/lint-test.yaml', 'Check smoke test results'],
    ['.github/workflows/cli-e2e-tests.yaml', 'Check CLI E2E test results'],
  ]) {
    const script = stepRun(file, name).replace(/\$\{\{ needs\.[^}]+\.result \}\}/g, '$RESULT');
    assert.ok(!script.includes('${{'), 'All workflow expressions must be resolved');
    for (const result of ['success', 'failure', 'cancelled', 'skipped', '']) {
      for (const allow of ['true', 'false']) {
        const expected = result === 'success' || (result === 'skipped' && allow === 'true');
        const actual = shell(script, { RESULT: result, ALLOW_SKIP: allow });
        assert.equal(actual.status === 0, expected, `${name}: ${result}, allow=${allow}`);
      }
    }
  }
});

test('an empty affected-package selection completes without scheduling shards', () => {
  const temporary = fs.mkdtempSync(path.join(os.tmpdir(), 'ci-empty-selection-'));
  try {
    const output = path.join(temporary, 'output');
    const result = shell(stepRun('.github/workflows/targeted-unit-tests.yaml', 'Choose between an inline run and shards'), {
      TEST_ARCHIVE: path.join(temporary, 'absent.tar.zst'), GITHUB_OUTPUT: output,
    });
    assert.equal(result.status, 0, result.stderr);
    assert.equal(fs.readFileSync(output, 'utf8').trim(), 'mode=none');
  } finally {
    fs.rmSync(temporary, { recursive: true });
  }
});

test('PostgreSQL checks TCP readiness and fails after bounded retries', () => {
  const temporary = fs.mkdtempSync(path.join(os.tmpdir(), 'ci-postgres-'));
  try {
    const trace = path.join(temporary, 'trace');
    fs.writeFileSync(path.join(temporary, 'docker'), '#!/bin/sh\nprintf "%s\\n" "$*" >> "$PROBE_TRACE"\ncase "$1" in run) echo test-postgres;; exec) exit "$PROBE_EXIT";; logs) exit 0;; esac\n', { mode: 0o700 });
    fs.writeFileSync(path.join(temporary, 'sleep'), '#!/bin/sh\nexit 0\n', { mode: 0o700 });
    for (const code of ['0', '1']) {
      fs.writeFileSync(trace, '');
      const result = shell(stepRun('.github/actions/postgres-start/action.yaml', 'Start database and wait for TCP readiness'), {
        PATH: `${temporary}:${process.env.PATH}`, PROBE_EXIT: code, PROBE_TRACE: trace,
      });
      assert.equal(result.status === 0, code === '0', result.stderr);
      const probes = fs.readFileSync(trace, 'utf8').split('\n').filter(line => line.startsWith('exec '));
      assert.equal(probes.length, code === '0' ? 1 : 30);
      assert.ok(probes.every(line => line === 'exec test-postgres pg_isready -h 127.0.0.1 -U postgres -t 1'));
    }
  } finally {
    fs.rmSync(temporary, { recursive: true });
  }
});

test('lint workflow reports measurements only for the instrumented unit-test job', () => {
  const lint = fs.readFileSync(path.join(root, '.github/workflows/lint-test.yaml'), 'utf8');
  assert.equal(lint.split('uses: ./.github/actions/ci-metrics-report').length - 1, 1);
  assert.match(lint, /uses: \.\/\.github\/actions\/ci-metrics-report\n\s+if: always\(\)\n\s+with:\n\s+name: rust-unit-tests/);
});

test('dynamic dependency check fails closed on resolution errors and banned packages', () => {
  const temporary = fs.mkdtempSync(path.join(os.tmpdir(), 'ci-dependency-check-'));
  try {
    fs.writeFileSync(path.join(temporary, 'cargo'), '#!/bin/sh\nprintf "%s\\n" "$PROBE_TREE"\nexit "$PROBE_EXIT"\n', { mode: 0o700 });
    for (const [tree, code, expected] of [
      ['aptos v1.0.0\nnot-openssl-sys v1.0.0', '0', true],
      ['openssl-sys v0.9.0', '0', false],
      ['pq-sys feature "default"', '0', false],
      ['', '101', false],
    ]) {
      const result = shell('bash crates/aptos/scripts/check_dynamic_deps.sh', {
        PATH: `${temporary}:${process.env.PATH}`, PROBE_TREE: tree, PROBE_EXIT: code,
      });
      assert.equal(result.status === 0, expected, result.stdout + result.stderr);
    }
  } finally {
    fs.rmSync(temporary, { recursive: true });
  }
});
