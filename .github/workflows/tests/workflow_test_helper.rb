# frozen_string_literal: true

require "json"
require_relative "../../actions/pr-ci-policy/lib/pr_ci_policy"

# Shared loaders and step finders for the workflow structure tests. Workflows and
# actions load through PrCiPolicy::SafeYaml, so the tests see the same data as the
# policy checker: no anchors, aliases or merge keys, and the root `on` key is a string.
module WorkflowTestHelper
  ROOT = File.expand_path("../../..", __dir__)

  module_function

  def load_workflow(name)
    PrCiPolicy::SafeYaml.load(File.read(File.join(ROOT, ".github", "workflows", name)), name)
  end

  def load_action(dir)
    PrCiPolicy::SafeYaml.load(File.read(File.join(ROOT, ".github", "actions", dir, "action.yml")), "#{dir}/action.yml")
  end

  def policy_manifest
    JSON.parse(File.read(File.join(ROOT, ".github", "ci", "pr-ci-policy.json")))
  end

  def docker_manifest
    JSON.parse(File.read(File.join(ROOT, ".github", "ci", "docker-capabilities.json")))
  end

  def report_producers
    JSON.parse(File.read(File.join(ROOT, ".github", "ci", "pr-ci-report-producers.json"))).fetch("producers")
  end

  def jobs(workflow)
    workflow.fetch("jobs")
  end

  def trigger(workflow)
    workflow.fetch("on")
  end

  def steps(job)
    job.fetch("steps")
  end

  def checkout_steps(job)
    steps(job).select { |step| step["uses"].to_s.start_with?("actions/checkout@") }
  end

  def exact_source_step(job)
    steps(job).find { |step| step["uses"].to_s.end_with?("/.github/actions/checkout-exact-pr-source") }
  end

  PR_TARGET = "github.event_name == 'pull_request_target'"

  # A label-gated PR workflow runs base-owned code with a read-only token, reruns
  # when labels or the head change, and cancels stale runs of the same PR.
  def assert_label_gated_pr_target(workflow, msg = nil)
    events = trigger(workflow)
    refute events.key?("pull_request"), msg
    assert_empty %w[labeled unlabeled opened synchronize reopened] - events.fetch("pull_request_target").fetch("types"), msg
    assert_equal({"contents" => "read", "pull-requests" => "read"}, workflow.fetch("permissions"), msg)
    assert_includes workflow.dig("concurrency", "group"), "github.event.pull_request.number", msg
    assert_equal true, workflow.dig("concurrency", "cancel-in-progress"), msg
  end

  # Only on pull_request_target, the job checks out base-owned code without
  # persisted credentials and runs compute-authorized for the PR; `approved`
  # reads that step. Returns the compute-authorized step.
  def assert_compute_authorization(job, msg = nil)
    checkout, authorize = steps(job)
    assert_equal 2, steps(job).length, msg
    assert checkout.fetch("uses").start_with?("actions/checkout@"), msg
    assert_includes [nil, "${{ github.repository }}"], checkout.dig("with", "repository"), msg
    assert_includes ["${{ github.sha }}", "${{ github.event.pull_request.base.sha }}"], checkout.dig("with", "ref"), msg
    assert_equal false, checkout.dig("with", "persist-credentials"), msg
    assert_equal "./.github/actions/compute-authorized", authorize.fetch("uses"), msg
    assert_equal "${{ github.event.pull_request.number }}", authorize.dig("with", "pr_number"), msg
    [checkout, authorize].each { |step| assert_equal PR_TARGET, step.fetch("if", job["if"]), msg }
    assert_includes job.dig("outputs", "approved"), "steps.#{authorize.fetch("id")}.outputs.approved", msg
    authorize
  end
end
