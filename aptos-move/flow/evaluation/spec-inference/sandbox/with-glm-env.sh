#!/bin/sh
# Load the study's Z.ai key and configure an Anthropic-compatible GLM session.
# This mirrors `/home/wrw/shared/bin/claude-alt glm` without printing or copying
# the credential into an evaluation artifact.

set -eu

# The credential may already be in the environment; otherwise read it from the
# key file. Either way it is never printed or written to a run artifact.
keys_file=${MOVE_INFERENCE_AI_KEYS_FILE:-"$HOME/.config/ai-keys.env"}
if [ -n "${ZAI_API_KEY:-}" ]; then
    :
elif [ -n "${GLM_TOKEN:-}" ]; then
    ZAI_API_KEY=$GLM_TOKEN
elif [ -n "${GML_TOKEN:-}" ]; then
    # The study machine's profile spells the variable `GML_TOKEN`.
    ZAI_API_KEY=$GML_TOKEN
elif [ -r "$keys_file" ]; then
    . "$keys_file"
else
    echo "with-glm-env: no credential; export GLM_TOKEN or ZAI_API_KEY, or provide $keys_file" >&2
    exit 1
fi
: "${ZAI_API_KEY:?empty credential}"

# Foundry and X-Api-Key settings take precedence over the bearer-token setup
# required by the Z.ai Anthropic-compatible endpoint.
unset CLAUDE_CODE_USE_FOUNDRY ANTHROPIC_FOUNDRY_RESOURCE ANTHROPIC_FOUNDRY_BASE_URL
unset ANTHROPIC_API_KEY

export ANTHROPIC_BASE_URL="https://api.z.ai/api/anthropic"
export ANTHROPIC_AUTH_TOKEN="$ZAI_API_KEY"
unset ZAI_API_KEY GLM_TOKEN GML_TOKEN MOONSHOT_API_KEY

export ANTHROPIC_MODEL="glm-5.3[1m]"
export ANTHROPIC_DEFAULT_OPUS_MODEL="$ANTHROPIC_MODEL"
export ANTHROPIC_DEFAULT_SONNET_MODEL="$ANTHROPIC_MODEL"
export ANTHROPIC_DEFAULT_FABLE_MODEL="$ANTHROPIC_MODEL"
export ANTHROPIC_DEFAULT_HAIKU_MODEL="glm-4.7"
export CLAUDE_CODE_SUBAGENT_MODEL="$ANTHROPIC_MODEL"
export CLAUDE_CODE_EFFORT_LEVEL="max"
export CLAUDE_CODE_AUTO_COMPACT_WINDOW="1000000"
export ENABLE_TOOL_SEARCH="false"
export CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC=1
export API_TIMEOUT_MS="3000000"

# Preflight must probe the same pinned Claude Code build mounted by the pilot
# sandbox, independently of the user's current `claude` symlink. The version has
# to match `claude_code_version` in the round's experiment configuration.
claude_version=${MOVE_INFERENCE_CLAUDE_VERSION:-2.1.258}
export MOVE_INFERENCE_CLAUDE_VERSION="$claude_version"
# The pin is on the build's version, not on where it is installed: a versioned
# directory and a native install are both acceptable, a wrong version is not.
# `MOVE_INFERENCE_CLAUDE_EXECUTABLE` names the build explicitly; otherwise the
# versioned path is tried first, then whatever `claude` resolves to.
claude_build="${MOVE_INFERENCE_CLAUDE_EXECUTABLE:-}"
if [ -z "$claude_build" ]; then
    versioned="$HOME/.local/share/claude/versions/$claude_version"
    if [ -x "$versioned" ]; then
        claude_build="$versioned"
    else
        claude_build=$(command -v claude 2>/dev/null || true)
    fi
fi
[ -n "$claude_build" ] && [ -x "$claude_build" ] || {
    echo "with-glm-env: no executable Claude Code build found for $claude_version" >&2
    exit 1
}
found_version=$("$claude_build" --version 2>/dev/null | awk '{print $1}')
[ "$found_version" = "$claude_version" ] || {
    echo "with-glm-env: Claude Code build at $claude_build reports ${found_version:-unknown}, pinned $claude_version" >&2
    exit 1
}
export CLAUDE_CODE_EXECUTABLE="$claude_build"

exec "$@"
