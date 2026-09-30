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
end
