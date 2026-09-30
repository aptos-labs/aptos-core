# frozen_string_literal: true

require "minitest/autorun"
require_relative "../workflow_test_helper"

# Ad-hoc Forge runs only on manual dispatch. A PR run left every input empty,
# so it never tested the input mapping below, and it gave cloud credentials to
# the checked-out PR code.
class AdhocForgeWorkflowTests < Minitest::Test
  include WorkflowTestHelper

  PATH = ".github/workflows/adhoc-forge.yaml"
  METADATA = "needs.determine-forge-run-metadata.outputs"

  def source
    File.read(File.join(ROOT, PATH))
  end

  def workflow
    @workflow ||= load_workflow("adhoc-forge.yaml")
  end

  def test_runs_only_on_manual_dispatch
    assert_equal ["workflow_dispatch"], trigger(workflow).keys
  end

  # The metadata job interpolates inputs.GIT_SHA into shell, so the guard treats
  # it as PR-controlled. It must hold no privilege.
  def test_only_the_forge_job_gets_the_callee_permissions
    assert_equal({"contents" => "read"}, workflow.fetch("permissions"))
    refute jobs(workflow).fetch("determine-forge-run-metadata").key?("permissions")
    metadata = PrCiPolicy::WorkflowAnalysis.new(PATH, source).jobs.fetch("determine-forge-run-metadata")
    assert_empty metadata.privileges.to_a

    callee = jobs(load_workflow("workflow-run-forge.yaml")).fetch("forge").fetch("permissions")
    assert_equal({"contents" => "read"}.merge(callee), jobs(workflow).fetch("adhoc-forge-test").fetch("permissions"))
  end

  def test_dispatch_inputs_reach_the_forge_callee_unchanged
    expected = {
      "GIT_SHA" => "${{ #{METADATA}.gitSha }}",
      "IMAGE_TAG" => "${{ #{METADATA}.imageTag }}",
      "FORGE_IMAGE_TAG" => "${{ #{METADATA}.forgeImageTag }}",
      "FORGE_IMAGE_NAME" => "${{ #{METADATA}.forgeImageName }}",
      "FORGE_TEST_SUITE" => "${{ #{METADATA}.forgeTestSuite }}",
      "FORGE_RUNNER_DURATION_SECS" => "${{ fromJSON(#{METADATA}.forgeRunnerDurationSecs) }}",
      "FORGE_CLUSTER_NAME" => "${{ #{METADATA}.forgeClusterName }}",
      "FORGE_NUM_VALIDATORS" => "${{ #{METADATA}.forgeNumValidators }}",
      "FORGE_NUM_VALIDATOR_FULLNODES" => "${{ #{METADATA}.forgeNumValidatorFullnodes }}",
      "FORGE_RETAIN_DEBUG_LOGS" => "${{ #{METADATA}.forgeRetainDebugLogs == 'true' }}",
      "FORGE_ENABLE_INDEXER" => "${{ #{METADATA}.forgeIndexerDeployerProfile != '' }}",
      "FORGE_DEPLOYER_PROFILE" => "${{ #{METADATA}.forgeIndexerDeployerProfile }}",
    }
    assert_equal expected, jobs(workflow).fetch("adhoc-forge-test").fetch("with")
  end

  # With the hardened manifest entry, the guard reports a changed hardened state.
  # Without it, the ratchet reports only :new_execution.
  def test_policy_rejects_restoring_the_pull_request_trigger
    head = source.sub(/^on:\n/, "on:\n  pull_request:\n    paths:\n      - \".github/workflows/adhoc-forge.yaml\"\n")
    violations = PrCiPolicy::PolicyChecker.new.check_pair(PATH, source, head)
    assert_includes violations.map { |v| [v.job, v.category] }, ["adhoc-forge-test", :hardened_state_changed]
  end
end
