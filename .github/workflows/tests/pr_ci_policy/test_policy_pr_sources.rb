# frozen_string_literal: true

require "minitest/autorun"
require_relative "policy_test_helper"
require_relative "property_support"

class PolicyPrSourcesTest < Minitest::Test
  include PolicyTestHelper
  include PolicyPropertySupport

  def test_privileged_source_fixture_classifications
    {
      "pr-title-run.yaml" => { "comment" => :execution_value },
      "shell-pr-expression.yaml" => { "execute" => :execution_value },
      "shell-pr-event-file.yaml" => { "execute" => :shell_source },
      "shell-pr-raw-sources.yaml" => %w[head-ref pull-ref gh-checkout git-fetch api-fetch api-list].to_h { |job| [job, :shell_source] },
      "script-pr-sources.yaml" => {
        "event-path-expression" => :execution_value,
        **%w[context-issue context-payload github-graphql-pull-request github-request-pulls github-rest-get-ref github-rest-pulls octokit-rest-pulls process-env-event-path].to_h { |job| [job, :script_api_source] },
      },
      "trusted-base-only.yaml" => { "trusted" => nil },
      "issue-api-pr-discovery.yaml" => { "execute" => nil },
    }.each do |name, jobs|
      actual = violations(nil, name).group_by(&:job).transform_values do |found|
        found.flat_map { |violation| violation.sources.map(&:kind) }.uniq.sort
      end
      expected = jobs.transform_values { |kind| [:pull_request_target_workflow, *kind].sort }

      assert_equal expected, actual, name
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

  def test_dash_suffixed_script_context_members_are_detected
    workflow = pr_target_workflow(
      permissions: { "contents" => "write" },
      steps: [{ "uses" => "actions/github-script@v7", "with" => { "script" => "const n = context.issue.number-0" } }],
    )

    result = PrCiPolicy::PolicyChecker.new.check_pair(PATH, nil, workflow)

    assert_includes result.flat_map { |violation| violation.sources.map(&:kind) }, :script_api_source
  end

  def test_only_accepts_required_event_exclusion_conjuncts
    # Only a required, syntactically valid AND conjunct excludes the event.
    render = lambda do |choice|
      member = ["github.event_name", "github['event_name']", "github [ 'event_name' ]"][choice[2] % 3]
      exclusion = "#{member} != 'pull_request_target'"
      rows = [
        [exclusion, true],
        ["#{exclusion} && (true || false)", true],
        ["#{exclusion} &&\n(true || false)", true],
        ["always() && #{member} == 'workflow_dispatch' && !cancelled()", true],
        ["#{exclusion} || github.actor == 'octocat'", false],
        ["always() && (#{exclusion} || true)", false],
        ["#{exclusion} && !!!", false],
        ["#{exclusion} && ${{ true }}", false],
        ["#{member} != \"pull_request_target\"", false],
        ["github.actor != 'octocat'", false],
        ["${{ #{exclusion} }}", true],
        [" ${{ #{exclusion} }}", false],
        ["${{ #{exclusion} }} ", false],
        ["text ${{ #{exclusion} }}", false],
        ["${{ #{exclusion} }", false],
      ]
      condition, excludes = rows.fetch(choice[0] % rows.size)
      # Templates (the last five rows) stay whole; outer parentheses would turn them into rendered text.
      depth = choice[0] % rows.size < rows.size - 5 ? choice[1] % 4 : 0
      ["(" * depth + condition + ")" * depth, excludes]
    end
    corpus = (0...15).to_a.product((0...4).to_a, (0...3).to_a)
    check_property("event exclusions", corpus: corpus,
                   generate: ->(random) { [random.rand(15), random.rand(4), random.rand(3)] }, describe: render) do |choice|
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

  def test_member_chain_length_fails_closed_on_non_ascii_bracket_member_names
    # Long s (U+017F) casefolds to ASCII s, but member lookup is ordinal.
    tokens = PrCiPolicy::Expression.tokenize("root['ſ']")

    assert_nil PrCiPolicy::Expression.member_chain_length(tokens, %w[root s])
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
