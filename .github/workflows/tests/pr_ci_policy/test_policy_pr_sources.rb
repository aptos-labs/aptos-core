# frozen_string_literal: true

require "minitest/autorun"
require_relative "policy_test_helper"

class PolicyPrSourcesTest < Minitest::Test
  include PolicyTestHelper

  def test_rejects_pull_request_title_in_shell_with_write_authority
    result = violations(nil, "pr-title-run.yaml").join("\n")

    assert_includes result, "PR-controlled execution value"
    assert_includes result, "write permission"
  end

  def test_rejects_github_event_number_in_privileged_execution
    result = violations(nil, "shell-pr-expression.yaml").join("\n")

    assert_includes result, "PR-controlled execution value"
    assert_includes result, "write permission"
  end

  def test_rejects_event_file_as_a_pr_source_with_privilege
    result = violations(nil, "shell-pr-event-file.yaml").join("\n")

    assert_includes result, "shell-derived PR source"
    assert_includes result, "write permission"
  end

  def test_rejects_raw_shell_pr_checkout_sources_with_privilege
    result = violations(nil, "shell-pr-raw-sources.yaml").join("\n")

    %w[head-ref pull-ref gh-checkout git-fetch api-fetch api-list].each do |job|
      assert_match(/job "#{job}":.*shell-derived PR source/, result)
    end
  end

  def test_member_access_rules_match_dot_and_bracket_forms
    workflow = <<~YAML
      name: reusable
      on: workflow_call
      permissions:
        contents: write
      jobs:
        call:
          runs-on: ubuntu-latest
          steps:
            - run: echo "$VALUE"
              env:
                VALUE: EXPRESSION
    YAML
    {
      "${{ github['event']['pull_request']['title'] }}" => [:execution_value],
      "${{ github.event['pull_request'].title }}" => [:execution_value],
      "${{ github [ 'event' ] . number }}" => [:execution_value],
      "${{ github['head_ref'] }}" => [:execution_value],
      "${{ inputs['head_sha'] }}" => [:execution_value],
      "context['payload']['pull_request'].number" => [:script_api_source],
      "github.rest['pulls'].get(context.repo)" => [:script_api_source],
      "${{ github['event']['pull_request']['base']['sha'] }}" => [],
    }.each do |expression, kinds|
      result = PrCiPolicy::PolicyChecker.new.check_pair(PATH, nil, workflow.sub("EXPRESSION", expression))

      assert_equal kinds, result.flat_map { |violation| violation.sources.map(&:kind) }.uniq, expression
    end
  end

  def test_reusable_source_outputs_match_job_ids_ending_in_a_dash
    workflow = <<~YAML
      name: caller
      on: pull_request_target
      permissions:
        contents: read
      jobs:
        call:
          if: github.event_name != 'pull_request_target'
          permissions:
            contents: write
          uses: ./.github/workflows/reusable.yaml
          with:
            SHA: ${{ needs.build-.outputs.sha }}
    YAML

    result = PrCiPolicy::PolicyChecker.new.check_pair(PATH, nil, workflow)

    assert_equal [:reusable_input], result.flat_map { |violation| violation.sources.map(&:kind) }.uniq
  end

  def test_member_access_dot_form_still_matches_before_a_trailing_dash
    # `-` is not part of a GitHub Actions expression identifier, but it is a
    # JavaScript subtraction operator, and this builder also feeds the
    # actions/github-script rules. `(?![\w-])` rejected a trailing dash;
    # `(?!\w)` is a `\b`-like boundary that still allows one.
    assert_match PrCiPolicy::WorkflowAnalysis.member_access("context", "issue", "number"), "context.issue.number-1"
    assert_match PrCiPolicy::WorkflowAnalysis.member_access("context", "issue", "number"), "context.issue.number- 1"
    assert_match PrCiPolicy::WorkflowAnalysis.member_access("context", "payload", "pull_request"), "context.payload.pull_request-0"
  end

  def test_dash_suffixed_script_context_members_are_detected
    workflow = pr_target_workflow(
      permissions: { "contents" => "write" },
      steps: [{ "uses" => "actions/github-script@v7", "with" => { "script" => "const n = context.issue.number-0" } }],
    )

    result = PrCiPolicy::PolicyChecker.new.check_pair(PATH, nil, workflow)

    assert_includes result.flat_map { |violation| violation.sources.map(&:kind) }, :script_api_source
    assert_includes result.join("\n"), "script/API-derived PR source"
  end

  def test_rejects_event_path_and_script_api_pr_sources_with_privilege
    result = violations(nil, "script-pr-sources.yaml").join("\n")

    assert_match(/job "event-path-expression":.*PR-controlled execution value/, result)
    %w[
      context-issue context-payload github-graphql-pull-request github-request-pulls
      github-rest-get-ref github-rest-pulls octokit-rest-pulls process-env-event-path
    ].each do |job|
      assert_match(/job "#{job}":.*script\/API-derived PR source/, result)
    end
  end

  def test_rejects_privileged_job_that_only_appears_to_execute_trusted_base_code
    result = violations(nil, "trusted-base-only.yaml").join("\n")

    assert_includes result, "pull_request_target workflow"
    assert_includes result, "write permission"
  end

  def test_rejects_indirect_issue_api_discovery_of_pr_code
    result = violations(nil, "issue-api-pr-discovery.yaml").join("\n")

    assert_includes result, "pull_request_target workflow"
    assert_includes result, "write permission"
  end

  def test_only_accepts_required_event_exclusion_conjuncts
    result = violations(nil, "pr-target-event-exclusions.yaml").join("\n")

    %w[
      unsafe-double-quoted unsafe-malformed-conjunct unsafe-mixed-template unsafe-nested-or
      unsafe-top-level-or unsafe-unrelated-condition
    ].each do |job|
      assert_match(/job "#{job}":.*pull_request_target workflow/, result)
    end
    %w[safe-not-target safe-nested-other-or safe-workflow-dispatch].each do |job|
      refute_match(/job "#{job}"/, result)
    end
  end

  def test_event_exclusion_fails_closed_on_non_ascii_event_name_literals
    write = { "contents" => "write" }
    {
      "!= with a Unicode-casefold lookalike" => "github.event_name != 'pull_requeſt_target'",
      "== with a non-ASCII literal" => "github.event_name == 'ｐush'",
    }.each do |description, condition|
      head = pr_target_workflow(job: { "if" => condition }, permissions: write)
      result = PrCiPolicy::PolicyChecker.new.check_pair(PATH, nil, head).join("\n")

      assert_includes result, "pull_request_target workflow", description
    end
  end

  def test_event_exclusion_rejects_template_with_surrounding_whitespace
    # GitHub evaluates a job `if:` as a whole expression only when the string
    # is exactly one ${{ }} template. With any surrounding text, even a space,
    # it renders a non-empty format() string, which is always true.
    write = { "contents" => "write" }
    template = "${{ github.event_name != 'pull_request_target' }}"
    {
      "leading space" => " #{template}",
      "trailing space" => "#{template} ",
    }.each do |description, condition|
      head = pr_target_workflow(job: { "if" => condition }, permissions: write)
      result = PrCiPolicy::PolicyChecker.new.check_pair(PATH, nil, head).join("\n")

      assert_includes result, "pull_request_target workflow", description
    end

    head = pr_target_workflow(job: { "if" => template }, permissions: write)
    refute_includes PrCiPolicy::PolicyChecker.new.check_pair(PATH, nil, head).join("\n"), "pull_request_target workflow"
  end

  def test_member_chain_length_fails_closed_on_non_ascii_bracket_member_names
    # 'ſ' (U+017F LATIN SMALL LETTER LONG S) is Unicode-casefold equal to 's',
    # so a naive casecmp? on the bracket string would treat this as a match
    # for the member "s" even though GitHub's own lookup is ordinal.
    tokens = PrCiPolicy::Expression.tokenize("root['ſ']")

    assert_nil PrCiPolicy::Expression.member_chain_length(tokens, %w[root s])
  end

  # Regression test for homoglyphs: the 'а' in 'event_nаme' is U+0430
  # CYRILLIC SMALL LETTER A. It does not test the case-folding fix, because
  # this member name is not casefold-equal to 'event_name' and fails to
  # match with or without that fix.
  def test_event_exclusion_ignores_homoglyph_bracket_event_name_member
    write = { "contents" => "write" }
    condition = "github['event_nаme'] != 'pull_request_target'"

    head = pr_target_workflow(job: { "if" => condition }, permissions: write)
    result = PrCiPolicy::PolicyChecker.new.check_pair(PATH, nil, head).join("\n")

    assert_includes result, "pull_request_target workflow"
  end
end
