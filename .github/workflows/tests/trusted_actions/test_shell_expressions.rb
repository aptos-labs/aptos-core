# frozen_string_literal: true

require "minitest/autorun"
require_relative "../workflow_test_helper"

# A `${{ }}` expression inside a shell script is substituted before the shell
# parses it, so attacker-controlled values become code. Hardened workflows and
# composite actions must pass values through `env` instead.
class ShellExpressionTest < Minitest::Test
  include WorkflowTestHelper

  # Known violations. Remove an entry when its scripts are fixed; the test fails
  # on any entry that no longer violates, so this list can only shrink.
  KNOWN_VIOLATIONS = %w[
    .github/workflows/adhoc-forge.yaml:determine-forge-run-metadata
    .github/workflows/cli-e2e-tests.yaml:run-cli-tests
    .github/workflows/forge-continuous-land-blocking-test.yaml:determine-docker-build-metadata
    .github/workflows/forge-continuous-land-blocking-test.yaml:fetch-last-released-docker-image-tag
    .github/workflows/forge-framework-upgrade.yaml:determine-test-metadata
    .github/workflows/forge-stable.yaml:determine-test-metadata
    .github/workflows/node-api-compatibility-tests.yaml:node-api-compatibility-tests
    .github/workflows/workflow-run-docker-rust-build.yaml:rust-all
    .github/workflows/workflow-run-forge.yaml:forge
    .github/actions/checkout-exact-pr-source/action.yml
    .github/actions/cli-rust-setup/action.yaml
    .github/actions/determine-or-use-target-branch-and-get-last-released-image/action.yaml
    .github/actions/docker-setup/action.yaml
    .github/actions/fullnode-sync/action.yaml
    .github/actions/get-latest-cli/action.yaml
    .github/actions/get-latest-docker-image-tag/action.yml
    .github/actions/install-grpcurl/action.yml
    .github/actions/move-prover-setup/action.yaml
    .github/actions/release-aptos-node/action.yml
    .github/actions/run-faucet-tests/action.yaml
    .github/actions/rust-setup/action.yaml
    .github/actions/wait-images-ci/action.yaml
  ].freeze

  def interpolates?(steps)
    steps.any? { |step| (step["run"] || step.dig("with", "command")).to_s.include?("${{") }
  end

  def scripts_by_owner
    workflows = policy_manifest.fetch("hardened_workflows").flat_map do |path|
      jobs(PrCiPolicy::SafeYaml.load(File.read(File.join(ROOT, path)), path)).map do |name, job|
        ["#{path}:#{name}", job.fetch("steps", [])]
      end
    end
    actions = Dir.glob(".github/actions/*/action.{yml,yaml}", base: ROOT).sort.map do |path|
      [path, PrCiPolicy::SafeYaml.load(File.read(File.join(ROOT, path)), path).fetch("runs").fetch("steps", [])]
    end
    workflows + actions
  end

  def test_shell_scripts_read_expressions_only_through_env
    violations = scripts_by_owner.select { |_, steps| interpolates?(steps) }.map(&:first)
    assert_empty violations - KNOWN_VIOLATIONS, "new `${{` in a shell script; pass the value through env"
    assert_empty KNOWN_VIOLATIONS - violations, "fixed; remove these from KNOWN_VIOLATIONS"
  end
end
