# frozen_string_literal: true

require "minitest/autorun"
require_relative "policy_test_helper"

class PolicyPrivilegesTest < Minitest::Test
  include PolicyTestHelper

  def test_accepts_new_secretless_pr_job
    assert_empty violations(nil, "new-safe-secretless.yaml")
  end

  def test_rejects_new_pr_job_with_secret
    assert_includes violations(nil, "new-unsafe-secret.yaml").join("\n"), "secret expression (secrets.DEPLOY_TOKEN)"
    assert_includes violations(nil, "secret-bracket.yaml").join("\n"), "secret expression (secrets.DEPLOY_TOKEN)"
  end

  def test_rejects_workflow_level_secret_inherited_by_pr_controlled_job
    result = violations(nil, "workflow-env-secret.yaml").join("\n")

    assert_includes result, "secret expression"
    assert_includes result, "PR-controlled checkout ref"
  end

  def test_accepts_job_override_of_workflow_level_secret
    assert_empty violations(nil, "workflow-env-secret-overridden.yaml")
  end

  def test_rejects_new_pr_job_with_write_permission
    assert_includes violations(nil, "new-unsafe-write.yaml").join("\n"), "write permission"
  end

  def test_rejects_new_pr_job_with_oidc_and_cloud_auth
    result = violations(nil, "new-unsafe-oidc-cloud.yaml").join("\n")
    assert_includes result, "id-token: write"
    assert_includes result, "cloud authentication"
    assert_includes result, "secret manager"
  end

  def test_accepts_privileged_privileges_only_behind_fixed_environment
    assert_empty violations(nil, "fixed-environment-privileged.yaml")
    refute_empty violations(nil, "new-unsafe-oidc-cloud.yaml")
  end

  def test_fixed_environment_never_allows_inherited_secrets_or_unsafe_permission_shapes
    assert_includes violations(nil, "fixed-environment-inherit.yaml").join("\n"), "inherited secrets"
    result = violations(nil, "fixed-environment-unsafe-permissions.yaml").join("\n")
    assert_includes result, "implicit permissions"
    assert_includes result, "dynamic permissions"
  end

  def test_tracks_cache_write_authority_across_workflow_and_job_scopes
    workflow = <<~YAML
      name: cache authority
      on: pull_request_target
      permissions:
        contents: read
      CACHE_MODE
      jobs:
        inspect:
          runs-on: ubuntu-latest
          JOB_CACHE_MODE
          ENVIRONMENT
          steps:
            - uses: actions/checkout@v4
              with:
                ref: ${{ github.event.pull_request.head.sha }}
    YAML

    cases = {
      "omitted mode is safe" => ["", "", "", false],
      "workflow read mode is safe" => ["cache-mode: read", "", "", false],
      "workflow none mode is safe" => ["cache-mode: none", "", "", false],
      "workflow write mode is unsafe" => ["cache-mode: write", "", "", true],
      "workflow write-only mode is unsafe" => ["cache-mode: write-only", "", "", true],
      "workflow dynamic mode is unsafe" => ["cache-mode: ${{ inputs.cache_mode }}", "", "", true],
      "workflow unknown mode is unsafe" => ["cache-mode: experimental", "", "", true],
      "job read overrides workflow write" => ["cache-mode: write", "cache-mode: read", "", false],
      "job write overrides workflow read" => ["cache-mode: read", "cache-mode: write", "", true],
      "job dynamic mode is unsafe" => ["cache-mode: none", "cache-mode: ${{ inputs.cache_mode }}", "", true],
      "approved environment does not suppress cache authority" => ["cache-mode: read", "cache-mode: write", "environment: privileged-pr-ci", true],
    }

    cases.each do |description, (workflow_mode, job_mode, environment, unsafe)|
      text = workflow
        .sub("CACHE_MODE", workflow_mode.empty? ? "" : "#{workflow_mode}\n")
        .sub("JOB_CACHE_MODE", job_mode.empty? ? "" : "#{job_mode}\n")
        .sub("ENVIRONMENT", environment.empty? ? "" : "#{environment}\n")
      analysis = PrCiPolicy::WorkflowAnalysis.new(PATH, text)
      kinds = analysis.jobs.fetch("inspect").effective_privileges.map(&:kind)

      if unsafe
        assert_includes kinds, :cache_write, description
      else
        refute_includes kinds, :cache_write, description
      end
    end
  end

  def test_ignores_identical_legacy_debt
    assert_empty violations("legacy-unsafe.yaml", "legacy-unsafe.yaml")
  end

  def test_rejects_whole_secrets_context_expressions
    workflow = <<~YAML
      name: whole secrets context
      on: pull_request_target
      permissions:
        contents: read
      jobs:
        inspect:
          runs-on: ubuntu-latest
          steps:
            - uses: actions/checkout@v4
              with:
                ref: ${{ github.event.pull_request.head.sha }}
            - run: ./inspect.sh
              env:
                TOKEN: SECRET_EXPRESSION
    YAML

    {
      "bare" => "${{ secrets }}",
      "grouped" => "${{ ( secrets ) }}",
      "nested function with case and spacing" => "${{ fromJSON( toJSON( SeCrEtS ) ) }}",
      "object filter" => "${{ secrets . * }}",
    }.each do |name, expression|
      result = PrCiPolicy::PolicyChecker.new.check_pair(PATH, nil, workflow.sub("SECRET_EXPRESSION", expression)).join("\n")

      assert_includes result, "secret expression (secrets context)", name
    end
  end

  def test_secret_scanning_time_grows_linearly_with_bracket_lookups
    cpu_seconds = lambda do |count|
      expression = "${{ #{(0...count).map { |index| "secrets['TOKEN_#{index}']" }.join(" && ")} }}"
      text = pr_target_workflow(steps: [{ "run" => "./inspect.sh", "env" => { "TOKENS" => expression } }])
      Array.new(3) do
        GC.start
        started = Process.clock_gettime(Process::CLOCK_PROCESS_CPUTIME_ID)
        analysis = PrCiPolicy::WorkflowAnalysis.new(PATH, text)
        elapsed = Process.clock_gettime(Process::CLOCK_PROCESS_CPUTIME_ID) - started
        assert_equal count, analysis.jobs.fetch("test").privileges.count { |privilege| privilege.kind == :secret }
        elapsed
      end.min
    end

    small = cpu_seconds.call(4_000)
    large = cpu_seconds.call(16_000)

    # Four times the input: a linear scan takes about 4x longer, a quadratic one about 16x.
    assert_operator large, :<, small * 10
  end

  def test_unparseable_expressions_fail_closed_as_secret_references
    {
      "double-quoted string" => "${{ \"secrets\" }}",
      "double quote hiding a later template" => "${{ \"x }} ${{ secrets.PROD_ADMIN }}",
      "unterminated string" => "${{ 'x }} ${{ secrets.PROD_ADMIN }}",
      "unterminated template" => "${{ secrets.PROD_ADMIN",
    }.each do |name, expression|
      head = pr_target_workflow(steps: [{ "run" => "./inspect.sh", "env" => { "TOKEN" => expression } }])
      result = PrCiPolicy::PolicyChecker.new.check_pair(PATH, nil, head)

      assert_equal [["test", :new_job, :secret, "unparseable expression"]],
                   result.select { |violation| violation.privilege.kind == :secret }
                         .map { |violation| [violation.job, violation.category, violation.privilege.kind, violation.privilege.detail] },
                   name
    end
  end

  def test_ignores_quoted_and_non_root_secrets_identifiers
    workflow = <<~YAML
      name: secret identifier controls
      on: pull_request_target
      permissions:
        contents: read
      jobs:
        inspect:
          runs-on: ubuntu-latest
          steps:
            - uses: actions/checkout@v4
              with:
                ref: ${{ github.event.pull_request.head.sha }}
            - run: ./inspect.sh
              env:
                TOKEN: SECRET_EXPRESSION
    YAML

    ["${{ 'secrets.PROD_ADMIN' }}", "${{ 'it''s secrets.PROD_ADMIN' }}", "${{ foo.secrets }}"].each do |expression|
      assert_empty PrCiPolicy::PolicyChecker.new.check_pair(PATH, nil, workflow.sub("SECRET_EXPRESSION", expression)), expression
    end
  end

  def test_accepts_whole_secrets_context_behind_approved_privileged_environment
    workflow = <<~YAML
      name: protected whole secrets context
      on: pull_request_target
      permissions:
        contents: read
      jobs:
        inspect:
          runs-on: ubuntu-latest
          environment: privileged-pr-ci
          steps:
            - uses: actions/checkout@v4
              with:
                ref: ${{ github.event.pull_request.head.sha }}
            - run: ./inspect.sh
              env:
                TOKEN: ${{ toJSON(secrets) }}
    YAML

    assert_empty PrCiPolicy::PolicyChecker.new.check_pair(PATH, nil, workflow)
  end

  def test_only_the_fixed_privileged_environment_gates_privileges
    {
      "fixed name" => ["privileged-pr-ci", []],
      "dynamic name" => ["${{ github.event.pull_request.head.ref }}", [["test", :new_job, :dynamic_environment]]],
      "unapproved mapping" => [{ "name" => "production" }, [["test", :new_job, :unapproved_environment]]],
      "unknown mapping key" => [
        { "name" => "privileged-pr-ci", "deployment" => false },
        [["test", :new_job, :unknown_environment_configuration]],
      ],
    }.each do |description, (environment, expected)|
      result = PrCiPolicy::PolicyChecker.new.check_pair(PATH, nil, pr_target_workflow(job: { "environment" => environment }))

      assert_equal expected, findings(result), description
    end
  end

  def test_existing_pr_controlled_job_may_not_gain_privilege_or_pr_sources
    legacy_secret = { "run" => "./deploy.sh", "env" => { "LEGACY_TOKEN" => "${{ secrets.LEGACY_TOKEN }}" } }
    with_env = ->(extra) { legacy_secret.merge("env" => legacy_secret.fetch("env").merge(extra)) }
    write = { "contents" => "write" }
    pr_controlled = [:checkout_ref, :execution_value, :pull_request_target_workflow]
    cases = {
      "new secret" => [
        pr_target_workflow(steps: [legacy_secret]),
        pr_target_workflow(steps: [with_env.call("PROD_ADMIN" => "${{ secrets.PROD_ADMIN }}")]),
        [["test", :privilege_increase, :secret, "secrets.PROD_ADMIN", pr_controlled]],
      ],
      "new secret after quoted closing braces" => [
        pr_target_workflow(steps: [legacy_secret]),
        pr_target_workflow(steps: [with_env.call("PROD_ADMIN" => "${{ format('{{0}}', secrets.PROD_ADMIN) }}")]),
        [["test", :privilege_increase, :secret, "secrets.PROD_ADMIN", pr_controlled]],
      ],
      "whole secrets context" => [
        pr_target_workflow(steps: [legacy_secret]),
        pr_target_workflow(steps: [with_env.call("ALL_SECRETS" => "${{ toJSON( secrets ) }}")]),
        [["test", :privilege_increase, :secret, "secrets context", pr_controlled]],
      ],
      "new id-token permission" => [
        pr_target_workflow(job: { "permissions" => { "contents" => "read" } }),
        pr_target_workflow(job: { "permissions" => { "contents" => "read", "id-token" => "write" } }),
        [["test", :privilege_increase, :id_token_write, nil, pr_controlled]],
      ],
      "new shell PR source" => [
        pr_target_workflow(permissions: write, steps: [{ "run" => "./legacy.sh" }]),
        pr_target_workflow(permissions: write, steps: [{ "run" => "gh pr checkout 42" }, { "run" => "./legacy.sh" }]),
        [["test", :new_source, :write_permission, "contents: write", (pr_controlled + [:shell_source]).sort]],
      ],
      "unchanged legacy debt" => [
        pr_target_workflow(permissions: write, steps: [legacy_secret]),
        pr_target_workflow(permissions: write, steps: [legacy_secret]),
        [],
      ],
    }

    cases.each do |description, (base, head, expected)|
      result = PrCiPolicy::PolicyChecker.new.check_pair(PATH, base, head)
      actual = result.map do |violation|
        [violation.job, violation.category, violation.privilege.kind, violation.privilege.detail, violation.sources.map(&:kind).sort]
      end

      assert_equal expected, actual, description
    end
  end

  def test_rejects_inherited_secrets_on_pr_controlled_reusable_call
    assert_includes violations(nil, "secrets-inherit.yaml").join("\n"), "inherited secrets"
  end

  def test_rejects_pr_controlled_reusable_call_with_write_authority
    result = violations(nil, "reusable-pr-controlled.yaml").join("\n")
    assert_includes result, "write permission"
    assert_includes result, "PR-controlled reusable workflow input"
  end

  def test_exact_source_wrappers_count_as_pr_controlled_checkouts
    workflow = <<~YAML
      name: reusable
      on: workflow_call
      permissions:
        contents: read
      jobs:
        build:
          runs-on: ubuntu-latest
          steps:
            - uses: ACTION
              with:
                source_repository: REPOSITORY
                source_sha: SHA
    YAML
    pr_inputs = ["${{ inputs.SOURCE_REPOSITORY }}", "${{ inputs.SOURCE_SHA }}"]
    both = %i[checkout_ref checkout_repository]
    {
      "exact-source action from trusted base" => ["./trusted-base/.github/actions/checkout-exact-pr-source", *pr_inputs, both],
      "exact-source action from the workspace" => ["./.github/actions/checkout-exact-pr-source", *pr_inputs, both],
      "privileged setup action" => ["./trusted-base/.github/actions/privileged-pr-setup", *pr_inputs, both],
      "case-variant path" => ["./trusted-base/.github/actions/Checkout-Exact-PR-Source", *pr_inputs, both],
      "trailing dot segment" => ["./trusted-base/.github/actions/checkout-exact-pr-source/.", *pr_inputs, both],
      "doubled separator before .github" => ["./trusted-base//.github/actions/checkout-exact-pr-source", *pr_inputs, both],
      "parent-directory segment that cancels out" => ["./trusted-base/.github/actions/pr-ci-policy/../checkout-exact-pr-source", *pr_inputs, both],
      "doubled trailing separator" => ["./trusted-base/.github/actions/checkout-exact-pr-source//", *pr_inputs, both],
      "PR head values" => [
        "./trusted-base/.github/actions/checkout-exact-pr-source",
        "${{ github.event.pull_request.head.repo.full_name }}",
        "${{ github.event.pull_request.head.sha }}",
        both + [:execution_value],
      ],
      "trusted base values" => ["./trusted-base/.github/actions/checkout-exact-pr-source", "${{ github.repository }}", "${{ github.sha }}", []],
    }.each do |description, (action, repository, sha, expected)|
      text = workflow.sub("ACTION", action).sub("REPOSITORY", repository).sub("SHA", sha)
      sources = PrCiPolicy::WorkflowAnalysis.new(PATH, text).jobs.fetch("build").sources

      assert_equal expected, sources.map(&:kind).sort, description
    end
  end

  def test_exact_source_wrappers_require_both_source_inputs
    workflow = <<~YAML
      name: reusable
      on: workflow_call
      jobs:
        build:
          runs-on: ubuntu-latest
          steps:
            - uses: ./trusted-base/.github/actions/checkout-exact-pr-source
              with:
                source_repository: ${{ inputs.SOURCE_REPOSITORY }}
    YAML

    error = assert_raises(PrCiPolicy::PolicyError) { PrCiPolicy::WorkflowAnalysis.new(PATH, workflow) }
    assert_includes error.message, "exact-source checkout source_sha must be a string"
  end

  def test_treats_unknown_checkout_repository_and_reusable_source_outputs_as_pr_controlled
    checkout = violations(nil, "dynamic-checkout-repository.yaml").join("\n")
    assert_includes checkout, "PR-controlled checkout repository"
    reusable = violations(nil, "reusable-needs-output.yaml").join("\n")
    assert_includes reusable, "PR-controlled reusable workflow input"
  end

  def test_allows_only_exact_approved_local_reusable_with_protected_callee
    callee = PrCiPolicy::WorkflowAnalysis.new("callee", fixture("protected-reusable-callee.yaml"))
    resolver = ->(_target) { callee }
    checker = PrCiPolicy::PolicyChecker.new

    assert_empty checker.check_pair(PATH, nil, fixture("approved-reusable-caller.yaml"), callee_resolver: resolver)
    refute_empty checker.check_pair(PATH, nil, fixture("approved-reusable-caller.yaml"))
    refute_empty checker.check_pair(PATH, nil, fixture("lookalike-reusable-caller.yaml"), callee_resolver: resolver)
  end

  def test_rejects_approved_reusable_when_callee_environment_is_dynamic
    callee = PrCiPolicy::WorkflowAnalysis.new("callee", fixture("dynamic-environment-callee.yaml"))
    resolver = ->(_target) { callee }
    result = PrCiPolicy::PolicyChecker.new.check_pair(
      PATH,
      nil,
      fixture("approved-reusable-caller.yaml"),
      callee_resolver: resolver,
    )
    refute_empty result
  end

  def test_rejects_shell_cloud_secret_manager_and_comment_authority
    result = violations(nil, "shell-authority.yaml").join("\n")
    assert_includes result, "cloud authentication"
    assert_includes result, "secret manager"
    assert_includes result, "repository dispatch/comment/write authority"
  end

  def test_permission_check_name_or_dependency_does_not_authorize_risk
    result = violations(nil, "permission-check-bypass.yaml").join("\n")
    assert_includes result, "write permission"
  end
end
