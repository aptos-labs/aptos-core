# frozen_string_literal: true

require "minitest/autorun"
require_relative "policy_test_helper"
require_relative "property_support"

class PolicyPrSourcesTest < Minitest::Test
  include PolicyTestHelper
  include PolicyPropertySupport

  def test_privileged_source_fixture_classifications
    {
      "pr-title-run.yaml" => { "comment" => "PR-controlled execution value" },
      "shell-pr-expression.yaml" => { "execute" => "PR-controlled execution value" },
      "shell-pr-event-file.yaml" => { "execute" => "shell-derived PR source" },
      "shell-pr-raw-sources.yaml" => %w[head-ref pull-ref gh-checkout git-fetch api-fetch api-list].to_h { |job| [job, "shell-derived PR source"] },
      "script-pr-sources.yaml" => {
        "event-path-expression" => "PR-controlled execution value",
        **%w[context-issue context-payload github-graphql-pull-request github-request-pulls github-rest-get-ref github-rest-pulls octokit-rest-pulls process-env-event-path].to_h { |job| [job, "script/API-derived PR source"] },
      },
      "trusted-base-only.yaml" => { "trusted" => "pull_request_target workflow" },
      "issue-api-pr-discovery.yaml" => { "execute" => "pull_request_target workflow" },
    }.each do |name, jobs|
      result = violations(nil, name).join("\n")
      jobs.each { |job, label| assert_match(/job "#{job}":.*#{Regexp.escape(label)}/, result, name) }
      assert_includes result, "write permission", name
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
    # GH Actions templates and JavaScript source strings have separate renderers.
    cases = [
      ["${{ github['event']['pull_request']['title'] }}", [:execution_value], %w[github event pull_request title], :actions],
      ["${{ github.event['pull_request'].title }}", [:execution_value], %w[github event pull_request title], :actions],
      ["${{ github [ 'event' ] . number }}", [:execution_value], %w[github event number], :actions],
      ["${{ github['head_ref'] }}", [:execution_value], %w[github head_ref], :actions],
      ["${{ inputs['head_sha'] }}", [:execution_value], %w[inputs head_sha], :actions],
      ["context['payload']['pull_request'].number", [:script_api_source], %w[context payload pull_request number], :script],
      ["github.rest['pulls'].get(context.repo)", [:script_api_source], %w[github rest pulls], :script],
      ["${{ github['event']['pull_request']['base']['sha'] }}", [], %w[github event pull_request base sha], :actions],
    ]
    render = lambda do |choice|
      original, _kinds, members, language = cases.fetch(choice[0] % cases.size)
      next original if choice[1].zero?
      space = " " * (choice[2] % 3)
      root, *tail = choice[3].odd? ? members.map(&:upcase) : members
      quote = language == :actions ? "'" : '"'
      chain = root + tail.each_with_index.map do |member, index|
        forms = ["#{space}.#{space}#{member}", "#{space}[#{space}'#{member}'#{space}]", index.even? ? "[#{quote}#{member}#{quote}]" : ".#{member}"]
        forms.fetch((choice[1] - 1) % forms.size)
      end.join
      language == :actions ? "${{ #{chain} }}" : "const value = #{chain};"
    end
    corpus = cases.each_index.to_a.product((0...4).to_a, (0...3).to_a, [0, 1])
    check_property("member syntax", corpus: corpus,
                   generate: ->(random) { [random.rand(cases.size), random.rand(4), random.rand(3), random.rand(2)] }, describe: render) do |choice|
      expression = render.call(choice)
      result = PrCiPolicy::PolicyChecker.new.check_pair(PATH, nil, workflow.sub("EXPRESSION", expression))
      assert_equal cases.fetch(choice[0] % cases.size)[1], result.flat_map { |violation| violation.sources.map(&:kind) }.uniq, expression
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
    # JavaScript subtraction must not hide a source member before the dash.
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
    # Only a required, syntactically valid AND conjunct excludes the event.
    render = lambda do |choice|
      member = ["github.event_name", "github['event_name']", "github [ 'event_name' ]"][choice[2] % 3]
      exclusion = "#{member} != 'pull_request_target'"
      condition, excludes = [
        ["#{exclusion} && (true || false)", true],
        ["always() && #{member} == 'workflow_dispatch' && !cancelled()", true],
        ["#{exclusion} || github.actor == 'octocat'", false],
        ["always() && (#{exclusion} || true)", false],
        ["#{exclusion} && !!!", false],
        ["#{exclusion} && ${{ true }}", false],
        ["#{member} != \"pull_request_target\"", false],
        ["${{ #{exclusion} }}", true],
        [" ${{ #{exclusion} }}", false],
        ["${{ #{exclusion} }} ", false],
        ["text ${{ #{exclusion} }}", false],
        ["${{ #{exclusion} }", false],
      ].fetch(choice[0] % 12)
      # Templates stay whole; outer parentheses would turn them into rendered text.
      depth = choice[0] % 12 < 7 ? choice[1] % 4 : 0
      ["(" * depth + condition + ")" * depth, excludes]
    end
    corpus = (0...12).to_a.product((0...4).to_a, (0...3).to_a)
    check_property("event exclusions", corpus: corpus,
                   generate: ->(random) { [random.rand(12), random.rand(4), random.rand(3)] }, describe: render) do |choice|
      condition, excludes = render.call(choice)
      assert_event_source(condition, !excludes, steps: [{ "run" => "./trusted.sh" }])
    end
  end

  def test_event_exclusion_fails_closed_on_non_ascii_event_name_literals
    { "!= with a Unicode-casefold lookalike" => "github.event_name != 'pull_requeſt_target'",
      "== with a non-ASCII literal" => "github.event_name == 'ｐush'" }.each do |description, condition|
      assert_event_source(condition, true, description: description)
    end
  end

  def test_event_exclusion_rejects_template_with_surrounding_whitespace
    # A template with surrounding text renders a nonempty, truthy string.
    template = "${{ github.event_name != 'pull_request_target' }}"
    { "leading space" => " #{template}", "trailing space" => "#{template} " }.each do |description, condition|
      assert_event_source(condition, true, description: description)
    end
    assert_event_source(template, false)
  end

  def test_member_chain_length_fails_closed_on_non_ascii_bracket_member_names
    # Long s (U+017F) casefolds to ASCII s, but member lookup is ordinal.
    tokens = PrCiPolicy::Expression.tokenize("root['ſ']")

    assert_nil PrCiPolicy::Expression.member_chain_length(tokens, %w[root s])
  end

  def test_event_exclusion_ignores_homoglyph_bracket_event_name_member
    # The Cyrillic a (U+0430) is not casefold-equal to the ASCII member.
    assert_event_source("github['event_nаme'] != 'pull_request_target'", true)
  end

  private

  def assert_event_source(condition, expected, description: condition, steps: nil)
    job = { "if" => condition }
    job["steps"] = steps if steps
    head = pr_target_workflow(job: job, permissions: { "contents" => "write" })
    result = PrCiPolicy::PolicyChecker.new.check_pair(PATH, nil, head).join("\n")
    assert_equal expected, result.include?("pull_request_target workflow"), description
  end
end
