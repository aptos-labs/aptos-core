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

  def test_dispatch_inputs_reach_the_forge_callee
    callee_inputs = trigger(load_workflow("workflow-run-forge.yaml")).fetch("workflow_call").fetch("inputs").keys
    dispatch_inputs = trigger(workflow).fetch("workflow_dispatch").fetch("inputs").keys
    metadata_outputs = jobs(workflow).fetch("determine-forge-run-metadata").fetch("outputs")
    metadata_outputs.each_value do |value|
      value.scan(/\binputs\.(\w+)/).flatten.each { |name| assert_includes dispatch_inputs, name, value }
    end
    with = jobs(workflow).fetch("adhoc-forge-test").fetch("with")
    refute_empty with
    with.each do |input, value|
      assert_includes callee_inputs, input
      references = value.scan(/#{Regexp.escape(METADATA)}\.(\w+)/).flatten
      refute_empty references, input
      references.each { |output| assert_includes metadata_outputs.keys, output, input }
    end
  end

  # Named regression: the guard rejects this change whether or not the file is hardened.
  def test_policy_rejects_restoring_the_pull_request_trigger
    head = source.sub(/^on:\n/, "on:\n  pull_request:\n    paths:\n      - \".github/workflows/adhoc-forge.yaml\"\n")
    violations = PrCiPolicy::PolicyChecker.new.check_pair(PATH, source, head)
    assert_includes violations.map(&:job), "adhoc-forge-test"
  end
end
