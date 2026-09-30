# frozen_string_literal: true

require "minitest/autorun"
require "open3"
require "tempfile"
require_relative "../workflow_test_helper"

class PrivilegedPrWorkflowTests < Minitest::Test
  include WorkflowTestHelper

  WORKFLOWS = {
    "workflow-run-docker-rust-publish-pr.yaml" => "publish-images",
    "workflow-run-pr-e2e-tests.yaml" => "prepare-images",
    "workflow-run-forge-pr.yaml" => "forge",
  }.freeze

  def forge_step(name)
    steps(jobs(load_workflow("workflow-run-forge-pr.yaml")).fetch("forge")).find { |step| step["name"] == name }
  end

  # Runs one workflow `run:` block with a scratch GITHUB_ENV and returns the lines it wrote.
  def run_step_env(step, env)
    Tempfile.create("github-env") do |file|
      _stdout, stderr, status = Open3.capture3(
        env.merge("PATH" => ENV.fetch("PATH"), "GITHUB_ENV" => file.path), "bash", "-c", step.fetch("run"),
      )
      assert status.success?, stderr
      File.read(file.path).lines(chomp: true)
    end
  end

  def test_each_privileged_job_has_a_fixed_environment_and_no_ambient_authority
    WORKFLOWS.each do |file, job_name|
      job = jobs(load_workflow(file)).fetch(job_name)
      assert_equal "privileged-pr-ci", job.fetch("environment"), file
      assert_equal({"contents" => "read", "id-token" => "write"}, job.fetch("permissions"), file)
      source = File.read(File.join(ROOT, ".github", "workflows", file))
      %w[GIT_CREDENTIALS cache-from cache-to secrets:\ inherit @main].each do |token|
        refute_includes source, token, file
      end
    end
  end

  def test_image_downloads_use_only_producer_artifact_ids
    %w[workflow-run-pr-e2e-tests.yaml workflow-run-forge-pr.yaml].each do |file|
      job_steps = steps(jobs(load_workflow(file)).values.first)
      downloads = job_steps.select { |step| step["uses"].to_s.start_with?("actions/download-artifact@") }
      assert_equal 1, downloads.length
      assert_equal "${{ inputs.IMAGE_MANIFEST_ID }}", downloads.first.dig("with", "artifact-ids")
      refute downloads.first.fetch("with").key?("name")
      id_check = job_steps.find { |step| step.dig("env", "IMAGE_MANIFEST_ID") }
      assert_includes id_check.fetch("run"), "^[1-9][0-9]*$"
      assert_operator job_steps.index(id_check), :<, job_steps.index(downloads.first)
      verify = job_steps.find { |step| step.dig("with", "mode") == "verify" }
      assert_equal "./trusted-base/.github/actions/protected-image-manifest", verify.fetch("uses")
      assert_equal "${{ inputs.SOURCE_SHA }}", verify.dig("with", "source_sha")
      assert_equal "${{ github.run_id }}", verify.dig("with", "run_id")
    end
  end

  def test_shell_commands_read_values_only_from_env
    WORKFLOWS.each do |file, job_name|
      steps(jobs(load_workflow(file)).fetch(job_name)).each do |step|
        script = step["run"] || step.dig("with", "command")
        refute_includes script.to_s, "${{", "#{file}: #{step["name"]}"
      end
    end
  end

  def test_pr_workflow_call_interfaces_do_not_accept_repository_secrets
    WORKFLOWS.each_key do |file|
      call = trigger(load_workflow(file)).fetch("workflow_call")
      refute call.key?("secrets"), file
      %w[SOURCE_REPOSITORY SOURCE_SHA PR_NUMBER BASE_SHA].each do |input|
        assert_equal true, call.fetch("inputs").fetch(input).fetch("required"), "#{file}: #{input}"
      end
    end
  end

  def test_forge_resolves_baseline_with_trusted_base_owned_mapping
    target = forge_step("Determine the trusted baseline branch")
    assert_equal "trusted-base/testsuite", target.fetch("working-directory")
    assert_includes target.fetch("run"), "determine_target_branch_to_fetch_last_released_image.py"
    refute_includes target.fetch("run"), "pr-source"

    baseline = forge_step("Resolve a trusted baseline without executing branch code")
    assert_equal "${{ steps.baseline-target.outputs.TARGET_BRANCH }}", baseline.fetch("env").fetch("TARGET_BRANCH")
  end

  def test_forge_namespace_derivation_is_deterministic_sha_scoped_and_bounded
    step = forge_step("Derive a bounded PR-and-SHA-scoped namespace")
    derive = lambda do |sha|
      lines = run_step_env(step, "PR_NUMBER" => "9999999999", "NAMESPACE_KIND" => "abcdefghijklmno", "SOURCE_SHA" => sha)
      lines.grep(/\AFORGE_NAMESPACE=/).first.delete_prefix("FORGE_NAMESPACE=")
    end

    first = derive.call("a" * 40)
    assert_equal first, derive.call("a" * 40)
    refute_equal first, derive.call("b" * 40)
    assert_operator first.length, :<=, 63
    assert_match(/\A[a-z0-9-]+\z/, first)
  end

  def test_forge_pins_every_image_role_to_an_explicit_tag
    step = forge_step("Pin all Forge image roles to explicit tags")
    pr_tag = "pr-7_#{"c" * 40}"
    assert_equal(
      ["IMAGE_TAG=#{pr_tag}", "UPGRADE_IMAGE_TAG=#{pr_tag}", "FORGE_IMAGE_TAG=#{pr_tag}", "FORGE_IMAGE_NAME=forge"],
      run_step_env(step, "USE_BASELINE_IMAGE" => "false", "BASELINE_IMAGE_TAG" => "", "PR_IMAGE_TAG" => pr_tag),
    )
    assert_equal(
      ["IMAGE_TAG=#{"d" * 40}", "UPGRADE_IMAGE_TAG=#{pr_tag}", "FORGE_IMAGE_TAG=#{pr_tag}", "FORGE_IMAGE_NAME=forge"],
      run_step_env(step, "USE_BASELINE_IMAGE" => "true", "BASELINE_IMAGE_TAG" => "d" * 40, "PR_IMAGE_TAG" => pr_tag),
    )
  end

  def test_protected_e2e_retains_retry_and_failure_log_behavior
    e2e_steps = steps(jobs(load_workflow("workflow-run-pr-e2e-tests.yaml")).fetch("e2e-tests"))
    retried = e2e_steps.select { |step| step["uses"] == "nick-fields/retry@ce71cc2ab81d554ebbe88c79ab5975992d79ba08" }
    assert_equal(
      {
        "Generate YAML API specification" => [3, 20],
        "Generate JSON API specification" => [3, 20],
        "Run CLI tests against devnet" => [5, 20],
        "Run CLI tests against testnet" => [5, 20],
        "Run CLI tests against mainnet" => [5, 20],
      },
      retried.to_h { |step| [step.fetch("name"), [step.dig("with", "max_attempts"), step.dig("with", "timeout_minutes")]] },
    )
    logs = e2e_steps.find { |step| step["if"].to_s.include?("failure()") }
    assert_includes logs.fetch("run"), "local-testnet-custom"
  end

  def test_published_tags_are_pr_scoped_and_include_the_full_sha
    publish = File.read(File.join(ROOT, ".github", "workflows", "workflow-run-docker-rust-publish-pr.yaml"))
    bake = File.read(File.join(ROOT, "docker", "builder", "docker-bake-rust-all.hcl"))
    assert_includes publish, "mode: publish-build"
    assert_includes publish, "source_sha: ${{ inputs.SOURCE_SHA }}"
    assert_includes bake, "${GCP_DOCKER_ARTIFACT_REPO}/${target}:${IMAGE_TAG_PREFIX}${GIT_SHA}"
    assert_includes bake, "${GCP_DOCKER_ARTIFACT_REPO}/${target}:${IMAGE_TAG_PREFIX}${NORMALIZED_GIT_BRANCH_OR_PR}"
  end

  # The branch output can come from github.head_ref, so it reaches shell only through env.
  def test_forge_stable_passes_the_test_branch_to_shell_through_env
    branch = "${{ steps.determine-test-branch.outputs.BRANCH }}"
    job_steps = steps(jobs(load_workflow("forge-stable.yaml")).fetch("determine-test-metadata"))

    job_steps.filter_map { |step| step["run"] }.each { |script| refute_includes script, "steps.determine-test-branch" }
    ["Hash the branch", "Write summary"].each do |name|
      assert_equal branch, job_steps.find { |step| step["name"] == name }.dig("env", "BRANCH"), name
    end
  end
end
