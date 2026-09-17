// Only GitHub's service may authenticate cache scope. A branch name in an S3 key
// does not prevent another runner with the same instance role from writing it.
function isGitHubCacheUrl(value) {
  try {
    const url = new URL(value);
    return url.protocol === 'https:' &&
      url.hostname.endsWith('.actions.githubusercontent.com') &&
      !url.username && !url.password && (!url.port || url.port === '443');
  } catch {
    return false;
  }
}

function canWriteSharedCache(event, ref) {
  return ref === 'refs/heads/main' && ['push', 'workflow_dispatch'].includes(event);
}

module.exports = { isGitHubCacheUrl, canWriteSharedCache };
