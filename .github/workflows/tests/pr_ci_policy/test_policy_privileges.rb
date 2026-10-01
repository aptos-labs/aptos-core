# frozen_string_literal: true

require "minitest/autorun"
require_relative "policy_test_helper"
require_relative "property_support"

class PolicyPrivilegesTest < Minitest::Test
  include PolicyTestHelper
  include PolicyPropertySupport

  # Byte-exact variants of shared, readable YAML fixtures.
  FIXTURE_VARIANTS = {
    "new-unsafe-secret.yaml" => ["secret-bracket.yaml", [
      ["unsafe-bracket-secret", "unsafe-secret"], ["./tests.sh", "./run-tests.sh"],
      ["          ref: ${{ github.event.pull_request.head.sha }}\n", "          ref: ${{ github.event.pull_request.head.sha }}\n          persist-credentials: false\n"],
      ["secrets['DEPLOY_TOKEN']", "secrets.DEPLOY_TOKEN"],
    ]],
    "workflow-env-secret-overridden.yaml" => ["workflow-env-secret.yaml", [
      ["name: workflow", "name: overridden workflow"],
      ["    steps:\n", "    env:\n      DEPLOY_TOKEN: harmless\n    steps:\n"],
    ]],
    "fixed-environment-inherit.yaml" => ["secrets-inherit.yaml", [
      ["name: inherit", "name: fixed-inherit"], ["  call:\n", "  call:\n    environment: privileged-pr-ci\n"],
    ]],
    "reusable-needs-output.yaml" => ["reusable-pr-controlled.yaml", [
      ["name: reusable", "name: reusable-needs"],
      ["GIT_REPOSITORY: ${{ github.event.pull_request.head.repo.full_name }}", "SOURCE_REPOSITORY: ${{ needs.resolve.outputs.repository }}"],
      ["GIT_SHA: ${{ github.event.pull_request.head.sha }}", "SOURCE_SHA: ${{ needs.resolve.outputs.sha }}"],
    ]],
    "lookalike-reusable-caller.yaml" => ["approved-reusable-caller.yaml", [["publish-pr.yaml", "publish-pr-evil.yaml"]]],
    "dynamic-environment-callee.yaml" => ["protected-reusable-callee.yaml", [
      ["protected-callee", "dynamic-callee"], ["environment: privileged-pr-ci", "environment: ${{ inputs.ENVIRONMENT }}"],
      ["      - uses: google-github-actions/auth@v2\n        with:\n          workload_identity_provider: ${{ secrets.PROVIDER }}\n", ""],
    ]],
  }.freeze

  def fixture(name)
    return super unless FIXTURE_VARIANTS.key?(name)

    source, edits = FIXTURE_VARIANTS.fetch(name)
    edits.reduce(super(source)) { |text, (from, to)| text.sub(from, to) }
  end

  def test_privilege_fixture_expectations
    {
      "new-safe-secretless.yaml" => [],
      "new-unsafe-secret.yaml" => ["secret expression (secrets.DEPLOY_TOKEN)"],
      "secret-bracket.yaml" => ["secret expression (secrets.DEPLOY_TOKEN)"],
      "workflow-env-secret.yaml" => ["secret expression", "PR-controlled checkout ref"],
      "workflow-env-secret-overridden.yaml" => [],
      "new-unsafe-write.yaml" => ["write permission"],
      "new-unsafe-oidc-cloud.yaml" => ["id-token: write", "cloud authentication", "secret manager"],
      "fixed-environment-privileged.yaml" => [],
      "fixed-environment-inherit.yaml" => ["inherited secrets"],
      "fixed-environment-unsafe-permissions.yaml" => ["implicit permissions", "dynamic permissions"],
      "secrets-inherit.yaml" => ["inherited secrets"],
      "reusable-pr-controlled.yaml" => ["write permission", "PR-controlled reusable workflow input"],
      "dynamic-checkout-repository.yaml" => ["PR-controlled checkout repository"],
      "reusable-needs-output.yaml" => ["PR-controlled reusable workflow input"],
      "shell-authority.yaml" => ["cloud authentication", "secret manager", "repository dispatch/comment/write authority"],
    }.each do |name, expected|
      result = violations(nil, name).join("\n")
      expected.empty? ? assert_empty(result, name) : expected.each { |message| assert_includes result, message, name }
    end
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

    cases = INHERITANCE_CACHE.map do |workflow_mode, job_mode, unsafe|
      [workflow_mode ? "cache-mode: #{workflow_mode}" : "",
       job_mode == :inherit ? "" : "cache-mode: #{job_mode}", "", unsafe]
    end
    cases << ["cache-mode: read", "cache-mode: write", "environment: privileged-pr-ci", true]
    cases.each do |workflow_mode, job_mode, environment, unsafe|
      description = [workflow_mode, job_mode, environment].inspect
      text = workflow
        .sub("CACHE_MODE", workflow_mode.empty? ? "" : "#{workflow_mode}\n")
        .sub("JOB_CACHE_MODE", job_mode.empty? ? "" : "#{job_mode}\n")
        .sub("ENVIRONMENT", environment.empty? ? "" : "#{environment}\n")
      analysis = PrCiPolicy::WorkflowAnalysis.new(PATH, text)
      kinds = analysis.jobs.fetch("inspect").effective_privileges.map(&:kind)

      assert_equal unsafe, kinds.include?(:cache_write), description
    end
  end

  def test_ignores_identical_legacy_debt
    assert_empty violations("legacy-unsafe.yaml", "legacy-unsafe.yaml")
  end

  def test_rejects_whole_secrets_context_expressions
    {
      "bare" => "${{ secrets }}",
      "grouped" => "${{ ( secrets ) }}",
      "nested function with case and spacing" => "${{ fromJSON( toJSON( SeCrEtS ) ) }}",
      "object filter" => "${{ secrets . * }}",
    }.each do |name, expression|
      result = PrCiPolicy::PolicyChecker.new.check_pair(PATH, nil, secret_context_workflow("whole secrets context", expression)).join("\n")

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
    ["${{ 'secrets.PROD_ADMIN' }}", "${{ 'it''s secrets.PROD_ADMIN' }}", "${{ foo.secrets }}"].each do |expression|
      assert_empty PrCiPolicy::PolicyChecker.new.check_pair(PATH, nil, secret_context_workflow("secret identifier controls", expression)), expression
    end
  end

  def test_accepts_whole_secrets_context_behind_approved_privileged_environment
    workflow = secret_context_workflow("protected whole secrets context", "${{ toJSON(secrets) }}", environment: true)
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
    base = pr_target_workflow(steps: [legacy_secret])
    pr_controlled = [:checkout_ref, :execution_value, :pull_request_target_workflow]
    {
      "new secret" => ["PROD_ADMIN", "${{ secrets.PROD_ADMIN }}", "secrets.PROD_ADMIN"],
      "new secret after quoted closing braces" => ["PROD_ADMIN", "${{ format('{{0}}', secrets.PROD_ADMIN) }}", "secrets.PROD_ADMIN"],
      "whole secrets context" => ["ALL_SECRETS", "${{ toJSON( secrets ) }}", "secrets context"],
    }.each do |name, (key, expression, detail)|
      step = legacy_secret.merge("env" => legacy_secret.fetch("env").merge(key => expression))
      result = PrCiPolicy::PolicyChecker.new.check_pair(PATH, base, pr_target_workflow(steps: [step]))
      actual = result.map { |violation| [violation.job, violation.category, violation.privilege.kind, violation.privilege.detail, violation.sources.map(&:kind).sort] }
      assert_equal [["test", :privilege_increase, :secret, detail, pr_controlled]], actual, name
    end
    write = { "contents" => "write" }
    {
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
        pr_target_workflow(permissions: write, steps: [legacy_secret]), [],
      ],
    }.each do |name, (base, head, expected)|
      result = PrCiPolicy::PolicyChecker.new.check_pair(PATH, base, head)
      actual = result.map { |violation| [violation.job, violation.category, violation.privilege.kind, violation.privilege.detail, violation.sources.map(&:kind).sort] }
      assert_equal expected, actual, name
    end
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
    exact = "./trusted-base/.github/actions/checkout-exact-pr-source"
    {
      "exact-source action from trusted base" => [exact, *pr_inputs, both],
      "exact-source action from the workspace" => ["./.github/actions/checkout-exact-pr-source", *pr_inputs, both],
      "privileged setup action" => ["./trusted-base/.github/actions/privileged-pr-setup", *pr_inputs, both],
      "case-variant path" => ["./trusted-base/.github/actions/Checkout-Exact-PR-Source", *pr_inputs, both],
      "trailing dot segment" => ["#{exact}/.", *pr_inputs, both],
      "doubled separator before .github" => ["./trusted-base//.github/actions/checkout-exact-pr-source", *pr_inputs, both],
      "parent-directory segment that cancels out" => ["./trusted-base/.github/actions/pr-ci-policy/../checkout-exact-pr-source", *pr_inputs, both],
      "doubled trailing separator" => ["#{exact}//", *pr_inputs, both],
      "PR head values" => [
        exact,
        "${{ github.event.pull_request.head.repo.full_name }}",
        "${{ github.event.pull_request.head.sha }}",
        both + [:execution_value],
      ],
      "trusted base values" => [exact, "${{ github.repository }}", "${{ github.sha }}", []],
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

  def test_permission_check_name_or_dependency_does_not_authorize_risk
    assert_includes violations(nil, "permission-check-bypass.yaml").join("\n"), "write permission"
  end

  # Each row specifies workflow/job inheritance and both ungated/gated outcomes.
  INHERITANCE_PERMISSIONS = [
    [nil, :inherit, [:implicit_permissions], [:implicit_permissions]],
    [{ "contents" => "read" }, :inherit, [], []],
    [{ "contents" => "write" }, :inherit, [:write_permission], []],
    [{ "contents" => "write" }, {}, [], []],
    [{ "contents" => "read" }, { "id-token" => "write" }, [:id_token_write], []],
    [{ "contents" => "write" }, nil, [:implicit_permissions], [:implicit_permissions]],
    ["write-all", :inherit, [:write_permission], []],
    ["${{ inputs.permissions }}", :inherit, [:dynamic_permissions], [:dynamic_permissions]],
    [{ "contents" => "read" }, { "contents" => "${{ inputs.level }}" }, [:dynamic_permissions], [:dynamic_permissions]],
  ].freeze
  INHERITANCE_CACHE = [
    [nil, :inherit, false], ["read", :inherit, false], ["none", :inherit, false],
    ["write", :inherit, true], ["write-only", :inherit, true],
    ["${{ inputs.cache_mode }}", :inherit, true], ["experimental", :inherit, true],
    ["write", "read", false], ["read", "write", true], ["none", "${{ inputs.cache_mode }}", true],
  ].freeze

  def test_permission_environment_secret_and_cache_inheritance_combinations
    limits = [INHERITANCE_PERMISSIONS.length, INHERITANCE_CACHE.length, 2, 4, 4, 5, 2]
    # Exhaust every security choice; generation varies job count, expression depth and syntax.
    corpus = (0...limits[0]).to_a.product((0...limits[1]).to_a, [0, 1], [0, 1, 2, 3]).map { |row| row + [0, 0, 0] }
    check_property("permission_environment_secret_cache", corpus: corpus,
      generate: ->(random) { limits.map { |limit| random.rand(limit) } },
      describe: ->(choices) { inheritance_case(choices).first.inspect }) do |choices|
      text, expected = inheritance_case(choices)
      result = PrCiPolicy::PolicyChecker.new.check_pair(PATH, nil, text)
      assert_equal expected, findings(result).sort
    end
  end

  private

  def inheritance_case(choices)
    permission, cache, protected, placement, count, depth, syntax = choices.zip([9, 10, 2, 4, 4, 5, 2]).map { |value, limit| value % limit }
    workflow_permission, job_permission, exposed, gated = INHERITANCE_PERMISSIONS.fetch(permission)
    workflow_cache, job_cache, cache_write = INHERITANCE_CACHE.fetch(cache)
    expression = "secrets.DEPLOY_TOKEN"
    depth.times { expression = "toJSON(#{expression})" }
    token = { "TOKEN" => "${{ #{expression} }}" }
    job = { "runs-on" => "ubuntu-latest", "steps" => [{ "uses" => "actions/checkout@v4", "with" => { "ref" => "${{ github.event.pull_request.head.sha }}" } }] }
    job["permissions"] = job_permission unless job_permission == :inherit
    job["cache-mode"] = job_cache unless job_cache == :inherit
    job["environment"] = "privileged-pr-ci" if protected == 1
    job["env"] = placement == 1 ? { "TOKEN" => "harmless" } : token if [1, 2].include?(placement)
    job["steps"] << { "run" => "./inspect.sh", "env" => token } if placement == 3
    names = Array.new(count + 1) { |index| "inspect_#{index}" }
    document = { "on" => "pull_request_target", "permissions" => workflow_permission, "jobs" => names.to_h { |name| [name, job] } }
    document["cache-mode"] = workflow_cache unless workflow_cache.nil?
    document["env"] = token if placement < 2
    kinds = (protected == 1 ? gated : exposed).dup
    kinds << :secret if protected.zero? && placement != 1
    kinds << :cache_write if cache_write
    expected = names.flat_map { |name| kinds.uniq.map { |kind| [name, :new_job, kind] } }.sort
    [syntax.zero? ? JSON.generate(document) : JSON.pretty_generate(document), expected]
  end

  def secret_context_workflow(name, expression, environment: false)
    <<~YAML.sub("ENVIRONMENT", environment ? "    environment: privileged-pr-ci\n" : "")
      name: #{name}
      on: pull_request_target
      permissions:
        contents: read
      jobs:
        inspect:
          runs-on: ubuntu-latest
      ENVIRONMENT    steps:
            - uses: actions/checkout@v4
              with:
                ref: ${{ github.event.pull_request.head.sha }}
            - run: ./inspect.sh
              env:
                TOKEN: #{expression}
    YAML
  end
end
