const assert = require('node:assert/strict');
const { test } = require('node:test');
const { isGitHubCacheUrl, canWriteSharedCache } = require('./cache-policy.cjs');

test('accepts native HTTPS endpoints and rejects proxies and lookalike hosts', () => {
  assert.ok(isGitHubCacheUrl('https://results-receiver.actions.githubusercontent.com/'));
  for (const url of [undefined, '', 'http://localhost:3000/',
    'http://results-receiver.actions.githubusercontent.com/',
    'https://results-receiver.actions.githubusercontent.com.attacker.example/',
    'https://not-actions.githubusercontent.com/',
    'https://user@results-receiver.actions.githubusercontent.com/',
    'https://results-receiver.actions.githubusercontent.com:8443/']) {
    assert.equal(isGitHubCacheUrl(url), false, String(url));
  }
});

test('only trusted main pushes and manual runs populate shared caches', () => {
  for (const event of ['push', 'workflow_dispatch']) {
    assert.ok(canWriteSharedCache(event, 'refs/heads/main'));
    assert.equal(canWriteSharedCache(event, 'refs/heads/feature'), false);
  }
  for (const event of ['pull_request', 'pull_request_target', 'workflow_run', 'issue_comment']) {
    assert.equal(canWriteSharedCache(event, 'refs/heads/main'), false);
    assert.equal(canWriteSharedCache(event, 'refs/pull/123/merge'), false);
  }
});
