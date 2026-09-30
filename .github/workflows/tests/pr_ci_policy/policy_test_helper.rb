# frozen_string_literal: true

require "json"
require_relative "../../../actions/pr-ci-policy/lib/pr_ci_policy"

module PolicyTestHelper
  ROOT = File.expand_path("../../../..", __dir__)
  FIXTURES = File.join(__dir__, "fixtures")
  PATH = ".github/workflows/example.yaml"
  POLICY_WORKFLOW_PATH = ".github/workflows/pr-ci-policy.yaml"

  def fixture(name)
    File.read(File.join(FIXTURES, name))
  end

  def violations(base_name, head_name, path: PATH)
    base = base_name && fixture(base_name)
    head = head_name && fixture(head_name)
    PrCiPolicy::PolicyChecker.new.check_pair(path, base, head)
  end

  # Builds a pull_request_target workflow whose single job checks out the PR
  # head. JSON is valid YAML and never emits anchors or aliases.
  def pr_target_workflow(job: {}, steps: [], permissions: { "contents" => "read" }, job_name: "test")
    checkout = { "uses" => "actions/checkout@v4", "with" => { "ref" => "${{ github.event.pull_request.head.sha }}" } }
    JSON.pretty_generate(
      "name" => "builder",
      "on" => "pull_request_target",
      "permissions" => permissions,
      "jobs" => { job_name => { "runs-on" => "ubuntu-latest", "steps" => [checkout, *steps] }.merge(job) },
    )
  end

  # Builds a workflow_run workflow whose single job runs `steps`. JSON is valid
  # YAML and never emits anchors or aliases.
  def workflow_run_workflow(steps, permissions: { "contents" => "write" })
    JSON.pretty_generate(
      "name" => "follow-up",
      "on" => { "workflow_run" => { "workflows" => ["CI"], "types" => ["completed"] } },
      "permissions" => permissions,
      "jobs" => { "follow-up" => { "runs-on" => "ubuntu-latest", "steps" => steps } },
    )
  end

  def findings(result)
    result.map { |violation| [violation.job, violation.category, violation.privilege.kind] }
  end

  def policy_workflow_text
    File.read(File.join(ROOT, POLICY_WORKFLOW_PATH))
  end

  def policy_workflow_violations(base, head)
    PrCiPolicy::PolicyChecker.new.check_pair(POLICY_WORKFLOW_PATH, base, head)
  end
end
