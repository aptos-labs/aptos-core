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

function shell(script, env) {
  return spawnSync('bash', ['-e', '-o', 'pipefail', '-c', script], {
    cwd: root, env: { ...process.env, ...env }, encoding: 'utf8',
  });
}

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

test('cache initialization failure disables the wrapper without failing the job', () => {
  const temporary = fs.mkdtempSync(path.join(os.tmpdir(), 'ci-cache-preflight-'));
  try {
    const output = path.join(temporary, 'env');
    fs.writeFileSync(path.join(temporary, 'sccache'), '#!/bin/sh\nexit "$PROBE_EXIT"\n', { mode: 0o700 });
    for (const code of ['0', '2']) {
      fs.writeFileSync(output, '');
      const result = shell(stepRun('.github/actions/sccache-setup/action.yaml', 'Check cache availability'), {
        PATH: `${temporary}:${process.env.PATH}`, PROBE_EXIT: code, GITHUB_ENV: output,
      });
      assert.equal(result.status, 0, result.stderr);
      assert.equal(fs.readFileSync(output, 'utf8'), code === '0' ? '' : 'RUSTC_WRAPPER=\n');
    }
  } finally {
    fs.rmSync(temporary, { recursive: true });
  }
});

test('compiler cache uses only GHA and benchmark readers cannot populate it', () => {
  const temporary = fs.mkdtempSync(path.join(os.tmpdir(), 'ci-cache-config-'));
  try {
    for (const write of ['true', 'false']) {
      for (const readOnly of ['true', 'false']) {
        const output = path.join(temporary, 'env');
        fs.writeFileSync(output, '');
        const result = shell(stepRun('.github/actions/sccache-setup/action.yaml', 'Configure compilation cache'), {
          CI_CACHE_WRITE: write, CACHE_READ_ONLY: readOnly, CACHE_NAMESPACE: 'test', GITHUB_ENV: output,
        });
        assert.equal(result.status, 0, result.stderr);
        const configuration = fs.readFileSync(output, 'utf8');
        assert.ok(configuration.includes('SCCACHE_MULTILEVEL_CHAIN=gha\n'));
        assert.ok(configuration.includes('SCCACHE_MULTILEVEL_WRITE_ERROR_POLICY=all\n'));
        const mode = write === 'true' && readOnly !== 'true' ? 'READ_WRITE' : 'READ_ONLY';
        assert.ok(configuration.includes(`SCCACHE_GHA_RW_MODE=${mode}\n`));
      }
    }
  } finally {
    fs.rmSync(temporary, { recursive: true });
  }
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
