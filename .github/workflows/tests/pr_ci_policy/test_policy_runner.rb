# frozen_string_literal: true

require "base64"
require "minitest/autorun"
require "open3"
require "rbconfig"
require_relative "../../../actions/pr-ci-policy/lib/pr_ci_policy"

class PolicyRunnerTest < Minitest::Test
  ACTION_DIR = File.expand_path("../../../actions/pr-ci-policy", __dir__)
  FIXTURES = File.join(__dir__, "fixtures")
  BASE_REPO = "aptos-labs/aptos-core"
  HEAD_REPO = "fork-owner/aptos-core"
  BASE_SHA = "a" * 40
  HEAD_SHA = "b" * 40

  class FakeApi
    attr_reader :requests

    def initialize(responses)
      @responses = responses
      @requests = []
    end

    def get_json(path, not_found: false, max_bytes: nil)
      @requests << [path, not_found, max_bytes]
      response = @responses.fetch(path) { raise PrCiPolicy::PolicyError, "unexpected API request: #{path}" }
      raise PrCiPolicy::PolicyError, response.fetch(:error) if response.is_a?(Hash) && response.key?(:error)
      return nil if response == :not_found && not_found
      raise PrCiPolicy::PolicyError, "GitHub API returned 404" if response == :not_found

      response
    end
  end

  def files_path(page = 1)
    "/repos/#{BASE_REPO}/pulls/77/files?per_page=100&page=#{page}"
  end

  def base_content_path(path)
    "/repos/#{BASE_REPO}/contents/#{path}?ref=#{BASE_SHA}"
  end

  def head_content_path(path)
    "/repos/#{HEAD_REPO}/contents/#{path}?ref=#{HEAD_SHA}"
  end

  def event(changed_files: 1)
    {
      "repository" => { "full_name" => BASE_REPO },
      "pull_request" => {
        "number" => 77,
        "changed_files" => changed_files,
        "base" => { "sha" => BASE_SHA, "repo" => { "full_name" => BASE_REPO } },
        "head" => { "sha" => HEAD_SHA, "repo" => { "full_name" => HEAD_REPO } },
      },
    }
  end

  def content_payload(text)
    {
      "type" => "file",
      "encoding" => "base64",
      "content" => Base64.strict_encode64(text),
      "size" => text.bytesize,
      "sha" => "c" * 40,
    }
  end

  def fixture(name)
    File.read(File.join(FIXTURES, name))
  end

  # A PR that adds one workflow whose head content is `text`.
  def added_workflow_api(text, path: ".github/workflows/new.yaml")
    FakeApi.new(
      files_path => [{ "filename" => path, "status" => "added" }],
      base_content_path(path) => :not_found,
      head_content_path(path) => content_payload(text),
    )
  end

  def test_uses_fork_repository_and_exact_base_and_head_shas
    path = ".github/workflows/new.yaml"
    api = added_workflow_api(fixture("new-safe-secretless.yaml"), path: path)

    assert_empty PrCiPolicy::Runner.new(api).check(event)
    assert_includes api.requests.map(&:first), base_content_path(path)
    assert_includes api.requests.map(&:first), head_content_path(path)
  end

  def test_fetches_approved_protected_callee_from_exact_fork_head_sha
    caller_path = ".github/workflows/caller.yaml"
    callee_path = ".github/workflows/workflow-run-docker-rust-publish-pr.yaml"
    api = FakeApi.new(
      files_path => [{ "filename" => caller_path, "status" => "added" }],
      base_content_path(caller_path) => :not_found,
      head_content_path(caller_path) => content_payload(fixture("approved-reusable-caller.yaml")),
      head_content_path(callee_path) => content_payload(fixture("protected-reusable-callee.yaml")),
    )

    assert_empty PrCiPolicy::Runner.new(api).check(event)
    assert_includes api.requests.map(&:first), head_content_path(callee_path)
  end

  def test_paginates_pr_file_listing
    first = Array.new(100) { |i| { "filename" => "docs/#{i}.md", "status" => "modified" } }
    second = [{ "filename" => "README.md", "status" => "modified" }]
    api = FakeApi.new(files_path(1) => first, files_path(2) => second)

    assert_empty PrCiPolicy::Runner.new(api).check(event(changed_files: 101))
    assert_equal [files_path(1), files_path(2)], api.requests.map(&:first)
  end

  def test_fails_closed_on_api_error
    api = FakeApi.new(files_path => { error: "rate limited" })

    assert_raises(PrCiPolicy::PolicyError) { PrCiPolicy::Runner.new(api).check(event) }
  end

  def test_fails_closed_on_truncated_file_listing
    api = FakeApi.new(files_path => [{ "filename" => "README.md", "status" => "modified" }])

    assert_raises(PrCiPolicy::PolicyError) { PrCiPolicy::Runner.new(api).check(event(changed_files: 2)) }
  end

  def test_fails_closed_on_duplicate_file_entries
    duplicate = { "filename" => "README.md", "status" => "modified" }
    api = FakeApi.new(files_path => [duplicate, duplicate.dup])

    assert_raises(PrCiPolicy::PolicyError) { PrCiPolicy::Runner.new(api).check(event(changed_files: 2)) }
  end

  def test_fails_closed_on_file_count_limit
    assert_raises(PrCiPolicy::PolicyError) do
      PrCiPolicy::Runner.new(FakeApi.new({})).check(event(changed_files: PrCiPolicy::Runner::MAX_CHANGED_FILES + 1))
    end
  end

  def test_fails_closed_on_workflow_count_limit
    files = Array.new(PrCiPolicy::Runner::MAX_WORKFLOW_FILES + 1) do |i|
      { "filename" => ".github/workflows/#{i}.yaml", "status" => "added" }
    end
    api = FakeApi.new(files_path => files)

    assert_raises(PrCiPolicy::PolicyError) { PrCiPolicy::Runner.new(api).check(event(changed_files: files.length)) }
  end

  def test_fails_closed_on_oversized_workflow
    api = added_workflow_api("x" * (PrCiPolicy::Runner::MAX_FILE_BYTES + 1))

    assert_raises(PrCiPolicy::PolicyError) { PrCiPolicy::Runner.new(api).check(event) }
  end

  def test_fails_closed_on_non_utf8_workflow
    api = added_workflow_api("name: \xFF\n".b)

    error = assert_raises(PrCiPolicy::PolicyError) { PrCiPolicy::Runner.new(api).check(event) }
    assert_match(/not valid UTF-8/, error.message)
  end

  def test_fails_closed_on_renamed_workflow
    file = { "filename" => ".github/workflows/new.yaml", "previous_filename" => ".github/workflows/old.yaml", "status" => "renamed" }
    api = FakeApi.new(files_path => [file])

    assert_raises(PrCiPolicy::PolicyError) { PrCiPolicy::Runner.new(api).check(event) }
  end

  def test_fails_closed_when_workflow_presence_contradicts_its_status
    path = ".github/workflows/new.yaml"
    text = fixture("new-safe-secretless.yaml")
    cases = {
      "added" => [/added workflow already exists at base SHA/, { base_content_path(path) => content_payload(text), head_content_path(path) => content_payload(text) }],
      "removed" => [/removed workflow still exists at head SHA/, { base_content_path(path) => content_payload(text), head_content_path(path) => content_payload(text) }],
    }
    cases.each do |status, (message, contents)|
      api = FakeApi.new({ files_path => [{ "filename" => path, "status" => status }] }.merge(contents))

      error = assert_raises(PrCiPolicy::PolicyError, status) { PrCiPolicy::Runner.new(api).check(event) }
      assert_match message, error.message, status
    end
  end

  def test_rejects_every_change_status_under_a_protected_prefix
    cases = {
      "addition" => { "filename" => ".github/actions/pr-ci-policy/new.rb", "status" => "added" },
      "modification" => { "filename" => ".github/actions/pr-ci-policy/lib/pr_ci_policy/runner.rb", "status" => "modified" },
      "removal" => { "filename" => ".github/actions/pr-ci-policy/action.yml", "status" => "removed" },
      "rename out" => {
        "filename" => ".github/actions/pr-ci-policy-disabled/action.yml",
        "previous_filename" => ".github/actions/pr-ci-policy/action.yml",
        "status" => "renamed",
      },
      "rename in" => {
        "filename" => ".github/actions/pr-ci-policy/action.yml",
        "previous_filename" => ".github/actions/pr-ci-policy-disabled/action.yml",
        "status" => "renamed",
      },
      "policy manifest" => { "filename" => ".github/ci/pr-ci-policy.json", "status" => "modified" },
    }

    cases.each do |description, file|
      api = FakeApi.new(files_path => [file])
      result = PrCiPolicy::Runner.new(api).check(event)

      assert_equal [:protected_runtime], result.map(&:category), description
      assert_equal [files_path], api.requests.map(&:first), description
    end
  end

  def test_rejects_new_files_under_every_manifest_prefix
    PrCiPolicy::Manifest.read.protected_runtime_prefixes.select { |prefix| prefix.end_with?("/") }.each do |prefix|
      file = { "filename" => "#{prefix}added-by-pr.txt", "status" => "added" }
      result = PrCiPolicy::Runner.new(FakeApi.new(files_path => [file])).check(event)

      assert_equal [:protected_runtime], result.map(&:category), prefix
    end
  end

  def test_rejects_change_to_exact_protected_image_tag_helper
    file = { "filename" => "docker/builder/image-tag-prefix.sh", "status" => "modified" }
    result = PrCiPolicy::Runner.new(FakeApi.new(files_path => [file])).check(event)

    assert_equal [:protected_runtime], result.map(&:category)
  end

  def test_protected_prefixes_match_whole_directory_names
    file = { "filename" => ".github/actions/compute-authorized-v2/action.yml", "status" => "added" }

    assert_empty PrCiPolicy::Runner.new(FakeApi.new(files_path => [file])).check(event)
  end

  def test_rejects_changes_to_privileged_wrapper_actions
    %w[docker-forge-pr-report indexer-processor-dispatch pr-ci-report].each do |name|
      file = { "filename" => ".github/actions/#{name}/action.yml", "status" => "modified" }
      result = PrCiPolicy::Runner.new(FakeApi.new(files_path => [file])).check(event)

      assert_equal [:protected_runtime], result.map(&:category), name
    end
  end

  def test_entrypoint_fails_closed_outside_pull_request_target
    _stdout, stderr, status = Open3.capture3(
      { "GITHUB_EVENT_NAME" => "push", "GITHUB_EVENT_PATH" => nil, "GITHUB_TOKEN" => nil },
      RbConfig.ruby,
      File.join(ACTION_DIR, "policy_checker.rb"),
    )

    assert_equal 1, status.exitstatus
    assert_includes stderr, "PR CI policy check failed closed: policy must run only for pull_request_target"
  end

  def test_action_passes_the_action_path_through_the_environment
    action = PrCiPolicy::SafeYaml.load(File.read(File.join(ACTION_DIR, "action.yml")), "action.yml")
    step = action.fetch("runs").fetch("steps").fetch(0)

    assert_equal %q{ruby "$GITHUB_ACTION_PATH/policy_checker.rb"}, step.fetch("run")
  end
end
