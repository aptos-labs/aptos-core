# frozen_string_literal: true

require "json"
require_relative "../../actions/pr-ci-policy/lib/pr_ci_policy"

# Shared loaders and step finders for the workflow structure tests. Workflows and
# actions load through PrCiPolicy::SafeYaml, so the tests see the same data as the
# policy checker: no anchors, aliases or merge keys, and the root `on` key is a string.
module WorkflowTestHelper
  ROOT = File.expand_path("../../..", __dir__)
  # Reviewed third-party action pins. Tests compare exact SHAs because GitHub
  # also resolves commits from forks of the action repository.
  PINS = {
    checkout: "actions/checkout@11d5960a326750d5838078e36cf38b85af677262",
    setup_python: "actions/setup-python@a26af69be951a213d495a4c3e4e4022e16d87065",
    gcp_auth: "google-github-actions/auth@c200f3691d83b41bf9bbd8638997a462592937ed",
    get_secretmanager_secrets: "google-github-actions/get-secretmanager-secrets@2b5f97c5a4b9c105e64646762ad4fc3f5128e6f5",
    docker_login: "docker/login-action@c94ce9fb468520275223c153574b00df6fe4bcc9",
    buildx: "docker/setup-buildx-action@8d2750c68a42422c14e847fe6c8ac0403b4cbd6f",
    setup_crane: "imjasonh/setup-crane@00c9e93efa4e1138c9a7a5c594acd6c75a2fbf0c",
    backport: "sorenlouv/backport-github-action@ad888e978060bc1b2798690dd9d03c4036560947",
    slack: "slackapi/slack-github-action@af78098f536edbc4de71162a307590698245be95",
    sticky_comment: "marocchino/sticky-pull-request-comment@39c5b5dc7717447d0cba270cd115037d32d28443",
    retry: "nick-fields/retry@ce71cc2ab81d554ebbe88c79ab5975992d79ba08",
  }.freeze

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
