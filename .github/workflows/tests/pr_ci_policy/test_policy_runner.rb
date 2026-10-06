# frozen_string_literal: true

require "base64"
require "minitest/autorun"
require "open3"
require "rbconfig"
require_relative "../../../actions/pr-ci-policy/lib/pr_ci_policy"
require_relative "policy_test_helper"
require_relative "property_support"

class PolicyRunnerTest < Minitest::Test
  include PolicyPropertySupport
  include PolicyTestHelper
  ACTION_DIR = File.expand_path("../../../actions/pr-ci-policy", __dir__)
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

  # A PR that adds one workflow whose head content is `text`.
  def added_workflow_api(text, path: ".github/workflows/new.yaml")
    FakeApi.new(
      files_path => [{ "filename" => path, "status" => "added" }],
      base_content_path(path) => :not_found,
      head_content_path(path) => content_payload(text),
    )
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

  FILE_COUNTS = [0, 1, 2, 99, 100, 101, 199, 200, 999, 1_000].freeze

  def listed_files(count)
    Array.new(count) do |i|
      # Keep the original one-file and 101-file examples in the corpus.
      path = (count == 1 || (count == 101 && i == 100)) ? "README.md" : "docs/#{i}.md"
      { "filename" => path, "status" => "modified" }
    end
  end

  def paginated_api(files)
    pages = files.each_slice(100).to_a
    pages << [] if files.length % 100 == 0
    FakeApi.new(pages.each_with_index.to_h { |page, i| [files_path(i + 1), page] })
  end

  def test_file_listing_pagination_counts_and_duplicates
    # Kinds: complete, truncated, surplus, duplicate.
    corpus = FILE_COUNTS.product((0...4).to_a, [0])
    corpus += [101, 199, 200, 999, 1_000].map { |count| [count, 3, 1] }
    check_property("file_listing_pagination_counts_and_duplicates", corpus: corpus,
                   generate: ->(r) { [r.rand(0..1_000), r.rand(4), r.rand(2)] }) do |count, kind, cross_page|
      count %= 1_001
      kind %= 4
      count = [count, 1].max if kind == 1
      count = [count, 999].min if kind == 2
      count = [count, cross_page.odd? ? 101 : 2].max if kind == 3
      files = listed_files(count + (kind == 1 ? -1 : kind == 2 ? 1 : 0))
      if kind == 3
        files[0] = { "filename" => "README.md", "status" => "modified" }
        files[cross_page.odd? ? 100 : 1] = files[0].dup
      end
      api = paginated_api(files)
      runner = PrCiPolicy::Runner.new(api)
      if kind.zero?
        assert_empty runner.check(event(changed_files: count))
      else
        error = assert_raises(PrCiPolicy::PolicyError) { runner.check(event(changed_files: count)) }
        assert_match(kind == 3 ? /duplicate paths/ : /truncated or changed/, error.message)
      end
      assert_equal (1..(files.length / 100 + 1)).map { |page| files_path(page) }, api.requests.map(&:first)
    rescue PrCiPolicy::PolicyError => error
      flunk("listing #{[count, kind, cross_page].inspect} failed: #{error.message}")
    end
  end

  def test_fails_closed_on_file_count_limit
    [-1, 1_001].each do |count|
      api = FakeApi.new({})
      error = assert_raises(PrCiPolicy::PolicyError) { PrCiPolicy::Runner.new(api).check(event(changed_files: count)) }
      assert_match(/invalid changed_files count/, error.message)
      assert_empty api.requests
    end
    api = paginated_api(listed_files(1_001))
    error = assert_raises(PrCiPolicy::PolicyError) { PrCiPolicy::Runner.new(api).check(event(changed_files: 1_000)) }
    assert_match(/listing exceeds count limit/, error.message)
    api = FakeApi.new(files_path => listed_files(101))
    error = assert_raises(PrCiPolicy::PolicyError) { PrCiPolicy::Runner.new(api).check(event(changed_files: 101)) }
    assert_match(/page exceeds page size/, error.message)
  end

  def test_fails_closed_on_workflow_count_limit
    [50, 51].each do |count|
      files = Array.new(count) { |i| { "filename" => ".github/workflows/#{i}.yaml", "status" => "added" } }
      responses = { files_path => files }
      if count <= 50
        files.each do |file|
          path = file.fetch("filename")
          responses[base_content_path(path)] = :not_found
          responses[head_content_path(path)] = content_payload(fixture("new-safe-secretless.yaml"))
        end
      end
      api = FakeApi.new(responses)
      runner = PrCiPolicy::Runner.new(api)
      if count <= 50
        assert_empty runner.check(event(changed_files: count))
      else
        error = assert_raises(PrCiPolicy::PolicyError) { runner.check(event(changed_files: count)) }
        assert_match(/too many workflow files/, error.message)
        assert_equal [files_path], api.requests.map(&:first)
      end
    end
  end

  def padded_workflow(name, bytes)
    text = fixture(name)
    "#{text}##{'x' * (bytes - text.bytesize - 1)}"
  end

  def test_fails_closed_on_oversized_workflow
    max = PrCiPolicy::Runner::MAX_FILE_BYTES
    [[max, nil], [max + 1, /workflow size is invalid/]].each do |bytes, message|
      runner = PrCiPolicy::Runner.new(added_workflow_api(padded_workflow("new-safe-secretless.yaml", bytes)))
      if message
        error = assert_raises(PrCiPolicy::PolicyError) { runner.check(event) }
        assert_match message, error.message
      else
        assert_empty runner.check(event)
      end
    end
  end

  def test_fails_closed_when_combined_workflow_data_exceeds_the_total_limit
    max = PrCiPolicy::Runner::MAX_FILE_BYTES
    full = PrCiPolicy::Runner::MAX_TOTAL_BYTES / max
    callee_path = ".github/workflows/workflow-run-docker-rust-publish-pr.yaml"
    {
      "changed workflows" => [full + 1, nil],
      "changed workflows plus fetched callee" => [full - 1, "approved-reusable-caller.yaml"],
    }.each do |label, (count, caller)|
      added = Array.new(count) { |i| [".github/workflows/#{i}.yaml", padded_workflow("new-safe-secretless.yaml", max)] }
      added << [".github/workflows/caller.yaml", fixture(caller)] if caller
      responses = { files_path => added.map { |path, _| { "filename" => path, "status" => "added" } } }
      added.each do |path, text|
        responses[base_content_path(path)] = :not_found
        responses[head_content_path(path)] = content_payload(text)
      end
      responses[head_content_path(callee_path)] = content_payload(padded_workflow("protected-reusable-callee.yaml", max))
      runner = PrCiPolicy::Runner.new(FakeApi.new(responses))

      error = assert_raises(PrCiPolicy::PolicyError, label) { runner.check(event(changed_files: added.length)) }
      assert_match(/combined workflow data exceeds size limit/, error.message, label)
    end
  end

  def test_fails_closed_on_non_utf8_workflow
    api = added_workflow_api("name: \xFF\n".b)

    error = assert_raises(PrCiPolicy::PolicyError) { PrCiPolicy::Runner.new(api).check(event) }
    assert_match(/not valid UTF-8/, error.message)
  end

  def test_fails_closed_on_renamed_workflow
    [[".github/workflows/new.yaml", ".github/workflows/old.yaml"],
     [".github/workflows/new.yaml", "docs/old.md"],
     ["docs/new.md", ".github/workflows/old.yaml"]].each do |path, previous|
      file = { "filename" => path, "previous_filename" => previous, "status" => "renamed" }
      error = assert_raises(PrCiPolicy::PolicyError) { PrCiPolicy::Runner.new(FakeApi.new(files_path => [file])).check(event) }
      assert_match(/workflow renames are ambiguous/, error.message)
    end
  end

  def test_workflow_status_and_presence_table
    path = ".github/workflows/new.yaml"
    text = fixture("new-safe-secretless.yaml")
    # Rows are [base absent/head absent, absent/present, present/absent, present/present].
    outcomes = {
      "added" => [/added workflow is absent at head SHA/, nil, /added workflow already exists at base SHA/, /added workflow already exists at base SHA/],
      "modified" => Array.new(3, /modified workflow is absent at base or head SHA/) + [nil],
      "removed" => [/removed workflow is absent at base SHA/, /removed workflow is absent at base SHA/, nil, /removed workflow still exists at head SHA/],
    }
    outcomes.keys.product((0...4).to_a).each do |status, presence|
      base, head = [presence >= 2, presence.odd?].map { |present| present ? content_payload(text) : nil }
      api = FakeApi.new(files_path => [{ "filename" => path, "status" => status }],
                        base_content_path(path) => base, head_content_path(path) => head)
      runner = PrCiPolicy::Runner.new(api)
      if (message = outcomes.fetch(status)[presence])
        error = assert_raises(PrCiPolicy::PolicyError) { runner.check(event) }
        assert_match message, error.message
      else
        assert_empty runner.check(event)
      end
      assert_equal [[files_path, false, 2 * 1024 * 1024],
                    [base_content_path(path), status == "added", PrCiPolicy::Runner::MAX_FILE_BYTES * 2],
                    [head_content_path(path), status == "removed", PrCiPolicy::Runner::MAX_FILE_BYTES * 2]], api.requests
    end
  end

  def test_protected_prefixes_match_whole_directory_names
    file = { "filename" => ".github/actions/compute-authorized-v2/action.yml", "status" => "added" }
    assert_empty PrCiPolicy::Runner.new(FakeApi.new(files_path => [file])).check(event)
    prefixes = [".github/actions/compute-authorized/", ".github/actions/pr-ci-policy/", ".github/ci/"]
    paths = prefixes.flat_map do |prefix|
      [["#{prefix}action.yml", true], ["#{prefix.delete_suffix('/')}-v2/action.yml", false], [prefix.delete_suffix('/'), false]]
    end
    paths += [["docker/builder/image-tag-prefix.sh", true], ["docker/builder/image-tag-prefix.sh.bak", false],
              ["docker/builder/image-tag-prefix.sh/nested", false]]
    manifest = PrCiPolicy::Manifest.new(hardened_workflows: Set[], approved_protected_reusables: Set[],
                                      protected_runtime_prefixes: (prefixes + ["docker/builder/image-tag-prefix.sh"]).to_set)
    corpus = paths.each_index.flat_map do |current|
      (0...3).map { |status| [current, status, 0, 0, 0] } + paths.each_index.map { |previous| [current, 3, previous, 0, 0] }
    end
    check_property("protected_path_lookalikes_and_rename_directions", corpus: corpus,
                   generate: ->(r) { [r.rand(paths.length), r.rand(4), r.rand(paths.length), r.rand(32), r.rand(5)] }) do |current, status_choice, previous, suffix, depth|
      path, protected = paths[current % paths.length]
      old_path, old_protected = paths[previous % paths.length]
      filename = suffix.zero? ? "action.yml" : "file-#{suffix}.rb"
      nested = "nested/" * (depth % 5) + filename
      path, old_path = [path, old_path].map { |value| value.sub("action.yml", nested) }
      status = %w[added modified removed renamed][status_choice % 4]
      file = { "filename" => path, "status" => status }
      file["previous_filename"] = old_path if status == "renamed"
      expected = protected || (status == "renamed" && old_protected) ? [:protected_runtime] : []
      api = FakeApi.new(files_path => [file])
      result = PrCiPolicy::Runner.new(api, manifest: manifest).check(event)
      assert_equal expected, result.map(&:category), file.inspect
      assert_equal [files_path], api.requests.map(&:first)
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

  def test_action_passes_inputs_through_the_environment_not_the_script
    action = PrCiPolicy::SafeYaml.load(File.read(File.join(ACTION_DIR, "action.yml")), "action.yml")
    step = action.fetch("runs").fetch("steps").fetch(0)

    refute_includes step.fetch("run"), "${{"
    assert_includes step.fetch("run"), "$GITHUB_ACTION_PATH/policy_checker.rb"
    assert_equal "${{ inputs.token }}", step.fetch("env").fetch("GITHUB_TOKEN")
  end
end
