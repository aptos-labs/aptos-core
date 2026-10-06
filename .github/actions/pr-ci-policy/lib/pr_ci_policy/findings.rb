# frozen_string_literal: true

require "set"

module PrCiPolicy
  # One policy fact about a job, in one of two dimensions. A source risk is a
  # reason the job runs PR-controlled code or data. A privilege risk is
  # authority the job holds. A job violates the policy only when it has both.
  # `detail` names the scope, secret, level, or environment for the kinds
  # whose label has a "%s" slot, and is nil otherwise.
  Risk = Data.define(:kind, :detail)

  class Risk
    SOURCE_LABELS = {
      checkout_ref: "PR-controlled checkout ref",
      checkout_repository: "PR-controlled checkout repository",
      execution_value: "PR-controlled execution value",
      pull_request_checkout_default: "pull_request checkout default",
      pull_request_target_workflow: "pull_request_target workflow",
      pull_request_workflow: "pull_request workflow content",
      reusable_input: "PR-controlled reusable workflow input",
      script_api_source: "script/API-derived PR source",
      shell_source: "shell-derived PR source",
      workflow_run_workflow: "workflow_run workflow",
    }.freeze
    PRIVILEGE_LABELS = {
      cache_write: "cache write authority",
      cloud_auth: "cloud authentication",
      dynamic_environment: "dynamic environment",
      dynamic_permissions: "dynamic permissions",
      id_token_write: "id-token: write",
      implicit_permissions: "implicit permissions",
      inherited_secrets: "inherited secrets",
      secret: "secret expression (%s)",
      secret_manager: "secret manager",
      unapproved_environment: "unapproved environment (%s)",
      unknown_environment_configuration: "unknown environment configuration",
      unknown_permission_level: "unknown permission level (%s)",
      write_authority: "repository dispatch/comment/write authority",
      write_permission: "write permission (%s)",
    }.freeze

    # The kind selects the dimension. An unknown kind would count as a
    # privilege, so a mistyped source kind would make a PR-controlled job look
    # trusted. Reject it here.
    def initialize(kind:, detail: nil)
      raise ArgumentError, "unknown risk kind: #{kind.inspect}" unless SOURCE_LABELS.key?(kind) || PRIVILEGE_LABELS.key?(kind)

      super
    end

    def source?
      SOURCE_LABELS.key?(kind)
    end

    def to_s
      label = SOURCE_LABELS.fetch(kind) { PRIVILEGE_LABELS.fetch(kind) }
      detail.nil? ? label : format(label, detail)
    end
  end

  # One policy finding. Job findings carry the job name, its source risks, and
  # one privilege risk. An upstream finding also carries `dependent`, the
  # privileged job that needs the changed job; its sources and privilege are
  # the dependent's. File findings carry only the path and category.
  Violation = Data.define(:path, :category, :job, :sources, :privilege, :dependent)

  class Violation
    JOB_CATEGORIES = {
      hardened_job_changed: "hardened workflow risky job changed",
      hardened_state_changed: "hardened workflow execution state changed",
      hardened_upstream_changed: "hardened workflow upstream of privileged job changed",
      new_execution: "new PR-controlled execution",
      new_job: "new PR-controlled job",
      new_source: "new PR-controlled source",
      privilege_increase: "privilege increase",
    }.freeze
    FILE_CATEGORIES = {
      policy_workflow_changed: "protected policy workflow must not be changed",
      protected_runtime: "protected policy runtime file must not be changed through pull requests",
      removed: "protected policy or hardened workflow must not be removed",
    }.freeze

    def initialize(path:, category:, job: nil, sources: nil, privilege: nil, dependent: nil) = super

    def to_s
      return "#{path}: #{FILE_CATEGORIES.fetch(category)}" if job.nil?

      combination = "#{sources.map(&:to_s).sort.join(", ")} with #{privilege}"
      return "#{path}: job #{job.inspect}: #{JOB_CATEGORIES.fetch(category)} combines #{combination}" if dependent.nil?

      "#{path}: job #{job.inspect}: #{JOB_CATEGORIES.fetch(category)} feeds job #{dependent.inspect}, which combines #{combination}"
    end
  end
end
