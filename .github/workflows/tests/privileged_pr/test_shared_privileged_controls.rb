# frozen_string_literal: true

require "minitest/autorun"
require "fileutils"
require "open3"
require "tmpdir"
require_relative "../workflow_test_helper"

class SharedPrivilegedControlTests < Minitest::Test
  include WorkflowTestHelper

  CHECKOUT_PIN = "actions/checkout@11d5960a326750d5838078e36cf38b85af677262"
  GCP_AUTH_PIN = "google-github-actions/auth@c200f3691d83b41bf9bbd8638997a462592937ed"
  DOCKER_LOGIN_PIN = "docker/login-action@c94ce9fb468520275223c153574b00df6fe4bcc9"
  BUILDX_PIN = "docker/setup-buildx-action@8d2750c68a42422c14e847fe6c8ac0403b4cbd6f"
  EXACT_SOURCE_ACTION = "./trusted-base/.github/actions/checkout-exact-pr-source"
  GCP_REGISTRY_ACTION = "./trusted-base/.github/actions/gcp-registry-auth"
  REFRESHED_GCP_REGISTRY_ACTION = "./trusted-refresh/.github/actions/gcp-registry-auth"
  SETUP_ACTION = "./trusted-base/.github/actions/privileged-pr-setup"
  RUNTIME_SETUP_ACTION = "./trusted-base/.github/actions/privileged-pr-runtime-setup"
  REGISTRIES = ["us-docker.pkg.dev", "us-west1-docker.pkg.dev"].freeze
  # file => [job, first step that runs PR code]
  PRIVILEGED_WORKFLOWS = {
    "workflow-run-docker-rust-publish-pr.yaml" => ["publish-images", "Rebuild and publish immutable PR images"],
    "workflow-run-pr-e2e-tests.yaml" => ["e2e-tests", "Verify checked-in API specifications"],
    "workflow-run-forge-pr.yaml" => ["forge", "Run pre-Forge checks with explicit image tags"],
  }.freeze

  def test_exact_source_action_fixes_checkout_controls
    action = load_action("checkout-exact-pr-source")
    assert_required_source_inputs(action)
    action_steps = action.dig("runs", "steps")
    checkouts = action_steps.select { |step| step["uses"].to_s.start_with?("actions/checkout@") }
    assert_equal 1, checkouts.length
    checkout = checkouts.first
    assert_equal CHECKOUT_PIN, checkout.fetch("uses")
    assert_equal "pr-source", action.fetch("inputs").fetch("path").fetch("default")
    assert_equal "1", action.fetch("inputs").fetch("fetch-depth").fetch("default")
    assert_equal(
      {
        "repository" => "${{ inputs.source_repository }}",
        "ref" => "${{ inputs.source_sha }}",
        "path" => "${{ inputs.path }}",
        "fetch-depth" => "${{ inputs.fetch-depth }}",
        "persist-credentials" => false,
      },
      checkout.fetch("with"),
    )
    index = action_steps.index(checkout)
    assert_match(/verify-source\.sh" validate/, action_steps.first.fetch("run"))
    assert_operator action_steps.length, :>=, 4
    assert_includes action_steps.fetch(index - 1).fetch("run"), "RUNNER_TEMP"
    assert_equal "${{ inputs.path }}", action_steps.fetch(index + 1).fetch("working-directory")
    assert_includes action_steps.fetch(index + 1).fetch("run"), "steps.stage-verifier.outputs.path"
  end

  def test_exact_source_verifier_rejects_bad_shas_and_pins_branch_to_head
    verifier = File.join(ROOT, ".github", "actions", "checkout-exact-pr-source", "verify-source.sh")
    Dir.mktmpdir("exact-source") do |repo|
      git!(repo, "init", "--quiet")
      git!(repo, "config", "user.email", "ci@example.invalid")
      git!(repo, "config", "user.name", "CI")
      File.write(File.join(repo, "tracked.txt"), "trusted fixture\n")
      git!(repo, "add", "tracked.txt")
      git!(repo, "commit", "--quiet", "-m", "fixture")
      sha = git!(repo, "rev-parse", "HEAD").strip
      run = ->(*args) { Open3.capture3({"GITHUB_RUN_ID" => "24680"}, "bash", verifier, *args, chdir: repo) }

      stdout, stderr, status = run.call("verify", sha)
      assert status.success?, "#{stdout}\n#{stderr}"
      assert_equal sha, git!(repo, "rev-parse", "HEAD").strip
      assert_equal "ci-pr-24680", git!(repo, "branch", "--show-current").strip

      ["abc123", "A" * 40, "g" * 40].each do |bad_sha|
        _stdout, bad_stderr, bad_status = run.call("validate", bad_sha)
        refute bad_status.success?, bad_sha
        assert_includes bad_stderr, "full lowercase commit SHA", bad_sha
      end

      _stdout, mismatch_stderr, mismatch_status = run.call("verify", sha.start_with?("0") ? "1" * 40 : "0" * 40)
      refute mismatch_status.success?
      assert_includes mismatch_stderr, "does not match"
    end
  end

  def test_staged_verifier_survives_replacing_a_root_checkout
    stage = load_action("checkout-exact-pr-source").dig("runs", "steps").find { |step| step["id"] == "stage-verifier" }
    Dir.mktmpdir("exact-source-stage") do |dir|
      action_dir = File.join(dir, "action")
      Dir.mkdir(action_dir)
      FileUtils.cp(File.join(ROOT, ".github/actions/checkout-exact-pr-source/verify-source.sh"), action_dir)
      output = File.join(dir, "step-output")
      stage_env = {"RUNNER_TEMP" => dir, "GITHUB_ACTION_PATH" => action_dir, "GITHUB_OUTPUT" => output}
      _stdout, stderr, status = Open3.capture3(stage_env, "bash", "-c", stage.fetch("run"))
      assert status.success?, stderr
      verifier = File.read(output).split("=", 2).last.strip
      FileUtils.rm_rf(action_dir)

      repo = File.join(dir, "replacement")
      Dir.mkdir(repo)
      git!(repo, "init", "--quiet")
      git!(repo, "config", "user.email", "ci@example.invalid")
      git!(repo, "config", "user.name", "CI")
      File.write(File.join(repo, "tracked.txt"), "replacement source\n")
      git!(repo, "add", "tracked.txt")
      git!(repo, "commit", "--quiet", "-m", "source")
      sha = git!(repo, "rev-parse", "HEAD").strip
      _stdout, verify_stderr, verify_status = Open3.capture3({"GITHUB_RUN_ID" => "24681"}, "bash", verifier, "verify", sha, chdir: repo)
      assert verify_status.success?, verify_stderr
      assert_equal sha, git!(repo, "rev-parse", "HEAD").strip
    end
  end

  def test_gcp_registry_action_has_fixed_auth_and_registry_boundary
    action = load_action("gcp-registry-auth")
    assert_equal %w[access_token_lifetime create_credentials_file service_account workload_identity_provider],
                 action.fetch("inputs").keys.sort
    assert_equal "false", action.fetch("inputs").fetch("create_credentials_file").fetch("default")
    action_steps = action.dig("runs", "steps")
    auth = action_steps.find { |step| step["id"] == "auth" }
    assert_equal GCP_AUTH_PIN, auth.fetch("uses")
    assert_equal "access_token", auth.fetch("with").fetch("token_format")
    assert_registry_logins(action_steps, "auth")
  end

  def test_privileged_setup_runs_every_pre_execution_control_in_order
    runtime = load_action("privileged-pr-runtime-setup")
    assert_required_source_inputs(runtime)
    assert_equal "composite", runtime.dig("runs", "using")
    action_steps = runtime.dig("runs", "steps")
    assert_equal 4, action_steps.length
    prerequisites, source, auth, tags = action_steps
    %w[GCP_WORKLOAD_IDENTITY_PROVIDER GCP_SERVICE_ACCOUNT_EMAIL GCP_DOCKER_ARTIFACT_REPO].each do |name|
      assert_includes prerequisites.fetch("run"), "test -n \"$#{name}\"", name
    end
    assert_equal EXACT_SOURCE_ACTION, source.fetch("uses")
    assert_equal({"source_repository" => "${{ inputs.source_repository }}", "source_sha" => "${{ inputs.source_sha }}"},
                 source.fetch("with"))
    assert_equal GCP_REGISTRY_ACTION, auth.fetch("uses")
    assert_equal(
      {
        "create_credentials_file" => false,
        "access_token_lifetime" => 5400,
        "workload_identity_provider" => "${{ inputs.workload_identity_provider }}",
        "service_account" => "${{ inputs.service_account }}",
      },
      auth.fetch("with"),
    )
    assert_equal "tags", tags.fetch("id")
    action_steps.filter_map { |step| step["run"] }.each { |script| refute_includes script, "${{" }
    build = load_action("privileged-pr-setup")
    assert_required_source_inputs(build)
    assert_equal "composite", build.dig("runs", "using")
    runtime_step, buildx = build.dig("runs", "steps")
    assert_equal 2, build.dig("runs", "steps").length
    assert_equal RUNTIME_SETUP_ACTION, runtime_step.fetch("uses")
    assert_equal "runtime", runtime_step.fetch("id")
    assert_equal BUILDX_PIN, buildx.fetch("uses")
    assert_equal false, buildx.dig("with", "keep-state")
    assert_equal true, buildx.dig("with", "cleanup")
    assert_equal false, buildx.dig("with", "cache-binary")
    assert_includes policy_manifest.fetch("protected_runtime_prefixes"), ".github/actions/privileged-pr-setup/"
    assert_includes policy_manifest.fetch("protected_runtime_prefixes"), ".github/actions/privileged-pr-runtime-setup/"
    assert_includes policy_manifest.fetch("protected_runtime_prefixes"), "docker/builder/image-tag-prefix.sh"
  end

  # The trusted setup computes the tag. The PR checkout's bake script must use
  # that exact prefix for every configured build variant.
  def test_privileged_setup_tag_matches_the_published_tag
    sha = "0123456789abcdef0123456789abcdef01234567"
    variants = docker_manifest.fetch("variants").map { |variant| [variant.fetch("profile"), variant.fetch("features")] }
    (variants + [["ci", "a,b"]]).each do |profile, features|
      Dir.mktmpdir("image-tag") do |dir|
        env = {"PR_NUMBER" => "42", "SOURCE_SHA" => sha, "PROFILE" => profile, "FEATURES" => features,
               "GITHUB_RUN_ID" => "24680", "GITHUB_RUN_ATTEMPT" => "2"}
        outputs, github_env = run_tag_step(dir, env)
        expected_prefix = "pr-42_"
        expected_prefix += "#{profile}_" unless profile == "release"
        expected_prefix += "#{features.gsub(/[^a-zA-Z0-9]/, "_")}_" unless features.empty?
        expected_prefix += "r24680-a2_"
        assert_equal expected_prefix, outputs.fetch("image_tag_prefix"), "#{profile}:#{features}"
        assert_equal "#{expected_prefix}#{sha}", outputs.fetch("image_tag"), "#{profile}:#{features}"
        assert_equal ["PR_IMAGE_TAG=#{outputs.fetch("image_tag")}"], github_env
        assert_equal expected_prefix, run_stubbed_bake(dir, sha, profile, features, expected_prefix)
      end
    end
  end

  def test_bake_script_preserves_an_explicit_empty_prefix
    Dir.mktmpdir("image-tag") do |dir|
      assert_equal "", run_stubbed_bake(dir, "a" * 40, "performance", "failpoints", "")
    end
  end

  def test_bake_script_uses_local_helper_when_prefix_is_unset
    Dir.mktmpdir("image-tag") do |dir|
      bake = File.join(dir, "docker-bake-rust-all.sh")
      File.write(bake, File.read(File.join(ROOT, "docker", "builder", "docker-bake-rust-all.sh")))
      File.write(File.join(dir, "image-tag-prefix.sh"), "image_tag_prefix() { printf 'from-local-helper_'; }\n")
      assert_equal "from-local-helper_", run_stubbed_bake(dir, "a" * 40, "performance", "consensus-only-perf-test", nil, bake)
    end
  end

  def test_privileged_setup_rejects_values_that_could_inject_environment_lines
    [{"PR_NUMBER" => "1\nX=1"}, {"PROFILE" => "release\nX=1"}, {"FEATURES" => "failpoints\nX=1"}].each do |override|
      Dir.mktmpdir("image-tag") do |dir|
        env = {"PR_NUMBER" => "42", "SOURCE_SHA" => "a" * 40, "PROFILE" => "release", "FEATURES" => "",
               "GITHUB_RUN_ID" => "24680", "GITHUB_RUN_ATTEMPT" => "2"}.merge(override)
        assert_nil run_tag_step(dir, env, expect_success: false), override.keys.first
      end
    end
  end

  def test_privileged_setup_accepts_every_manifest_feature_set
    docker_manifest.fetch("variants").each do |variant|
      Dir.mktmpdir("image-tag") do |dir|
        env = {"PR_NUMBER" => "42", "SOURCE_SHA" => "a" * 40, "PROFILE" => variant.fetch("profile"),
               "FEATURES" => variant.fetch("features"), "GITHUB_RUN_ID" => "24680", "GITHUB_RUN_ATTEMPT" => "2"}
        run_tag_step(dir, env)
      end
    end
  end

  def test_image_tag_prefix_helper_can_be_sourced_without_side_effects
    helper = File.join(ROOT, "docker/builder/image-tag-prefix.sh")
    stdout, stderr, status = Open3.capture3("bash", "-c", 'source "$1"; image_tag_prefix 42 performance "a,b"', "bash", helper)
    assert status.success?, stderr
    assert_equal "pr-42_performance_a_b_", stdout
  end

  def test_callers_load_base_owned_actions_before_pr_execution
    local_workflow = load_workflow("workflow-run-docker-rust-build-pr.yaml")
    assert_equal true, trigger(local_workflow).fetch("workflow_call").fetch("inputs").fetch("BASE_SHA").fetch("required")
    local_job = jobs(local_workflow).fetch("build-local-images")
    assert_equal({"contents" => "read"}, local_job.fetch("permissions"))
    refute local_job.key?("environment")
    trusted_index = assert_trusted_checkout(local_job)
    source = exact_source_step(local_job)
    assert_equal EXACT_SOURCE_ACTION, source.fetch("uses")
    assert_equal({"source_repository" => "${{ inputs.SOURCE_REPOSITORY }}", "source_sha" => "${{ inputs.SOURCE_SHA }}"},
                 source.fetch("with"))
    assert_operator trusted_index, :<, steps(local_job).index(source)

    PRIVILEGED_WORKFLOWS.each do |file, (job_name, first_source_step)|
      job = jobs(load_workflow(file)).fetch(job_name)
      job_steps = steps(job)
      trusted_index = assert_trusted_checkout(job)
      expected_setup = file == "workflow-run-docker-rust-publish-pr.yaml" ? SETUP_ACTION : RUNTIME_SETUP_ACTION
      setups = job_steps.select { |step| step["uses"] == expected_setup }
      assert_equal 1, setups.length, file
      setup = setups.first
      assert_equal "setup", setup.fetch("id"), file
      %w[SOURCE_REPOSITORY SOURCE_SHA PR_NUMBER].each do |input|
        assert_equal "${{ inputs.#{input} }}", setup.dig("with", input.downcase), "#{file}: #{input}"
      end
      assert_nil exact_source_step(job), file
      registry_refreshes = job_steps.select { |step| step["uses"] == REFRESHED_GCP_REGISTRY_ACTION }
      assert_equal(file == "workflow-run-pr-e2e-tests.yaml" ? 0 : 1, registry_refreshes.length, file)

      execution = job_steps.find { |step| step["name"] == first_source_step }
      assert_operator trusted_index, :<, job_steps.index(setup), file
      assert_operator job_steps.index(setup), :<, job_steps.index(execution), file
      assert_equal "pr-source", execution.fetch("working-directory"), file
      if file == "workflow-run-docker-rust-publish-pr.yaml"
        assert_equal "${{ steps.setup.outputs.image_tag_prefix }}", execution.dig("env", "IMAGE_TAG_PREFIX"), file
        assert_operator job_steps.index(execution), :<, job_steps.index(registry_refreshes.first), file
      else
        refute job_steps.any? { |step| step["run"].to_s.include?("docker-bake-rust-all.sh") }, file
        registry_refreshes.each { |step| assert_operator job_steps.index(step), :<, job_steps.index(execution), file }
      end
      refute execution.fetch("env", {}).key?("CUSTOM_IMAGE_TAG_PREFIX"), file
      # No step may load PR-owned action code: the job mints registry credentials
      # before PR code runs, so a "./pr-source/..." step would hand it those credentials.
      refute job_steps.any? { |step| step["uses"].to_s.start_with?("./pr-source/") }, file
      # Before PR code runs, local actions must come from an exact trusted checkout.
      before_execution = job_steps.take(job_steps.index(execution))
      before_execution.select { |step| step["uses"].to_s.start_with?("./") }.each do |step|
        assert_match %r{\A\./trusted-(?:base|refresh)/}, step.fetch("uses"), "#{file}: #{step["name"]}"
      end
    end
  end

  # Forge consumes the protected producer manifest. It must not discover recent
  # images, wait for images from other runs, or post PR comments.
  def test_forge_uses_only_its_own_protected_images
    source = load_workflow("workflow-run-forge-pr.yaml").to_s
    %w[find_recent_images wait-images-ci sticky-pull-request-comment].each { |text| refute_includes source, text }
  end

  def test_callers_bind_the_event_base_sha
    callers = jobs(load_workflow("docker-build-test.yaml")).select do |_name, job|
      (PRIVILEGED_WORKFLOWS.keys + ["workflow-run-docker-rust-build-pr.yaml"]).include?(File.basename(job["uses"].to_s))
    end
    refute_empty callers
    callers.each do |name, job|
      assert_equal "${{ github.event.pull_request.base.sha }}", job.fetch("with").fetch("BASE_SHA"), name
    end
  end

  def test_forge_refresh_reuses_the_registry_action_before_pr_scripts
    job_steps = steps(jobs(load_workflow("workflow-run-forge-pr.yaml")).fetch("forge"))
    auth = job_steps.find { |step| step["id"] == "gcp-forge-auth" }
    assert_equal REFRESHED_GCP_REGISTRY_ACTION, auth.fetch("uses")
    refreshed = job_steps.find { |step| step["name"] == "Refresh exact trusted base actions" }
    assert_equal CHECKOUT_PIN, refreshed.fetch("uses")
    assert_equal "${{ inputs.BASE_SHA }}", refreshed.dig("with", "ref")
    assert_equal "trusted-refresh", refreshed.dig("with", "path")
    assert_equal true, auth.dig("with", "create_credentials_file")
    assert_equal "${{ steps.auth-duration.outputs.value }}", auth.dig("with", "access_token_lifetime")
    assert_equal "${{ secrets.GCP_WORKLOAD_IDENTITY_PROVIDER }}", auth.dig("with", "workload_identity_provider")
    assert_equal "${{ secrets.GCP_SERVICE_ACCOUNT_EMAIL }}", auth.dig("with", "service_account")
    setup_index = job_steps.index { |step| step["uses"] == RUNTIME_SETUP_ACTION }
    auth_index = job_steps.index(auth)
    forge_index = job_steps.index { |step| step["name"] == "Run pre-Forge checks with explicit image tags" }
    assert_operator setup_index, :<, auth_index
    assert_operator setup_index, :<, job_steps.index(refreshed)
    assert_operator job_steps.index(refreshed), :<, auth_index
    assert_operator auth_index, :<, forge_index
    assert_empty job_steps.select { |step| step["uses"].to_s.start_with?("docker/login-action@") }
    token_export = job_steps.find { |step| step.dig("env", "ACCESS_TOKEN") == "${{ steps.gcp-forge-auth.outputs.access_token }}" }
    refute_nil token_export
    assert_includes token_export.fetch("run"), "CLOUDSDK_AUTH_ACCESS_TOKEN=$ACCESS_TOKEN"
    assert_operator auth_index, :<, job_steps.index(token_export)
    assert_operator job_steps.index(token_export), :<, forge_index
  end

  private

  def run_stubbed_bake(dir, sha, profile, features, prefix, bake = File.join(ROOT, "docker", "builder", "docker-bake-rust-all.sh"))
    bin = File.join(dir, "bin")
    Dir.mkdir(bin)
    File.write(File.join(bin, "git"), "#!/bin/sh\n[ \"$1\" = rev-parse ] && echo #{sha}\nexit 0\n")
    File.write(File.join(bin, "docker"),
               "#!/bin/sh\n[ \"$1 $2\" = 'buildx bake' ] && printf '%s' \"$IMAGE_TAG_PREFIX\" > \"$PREFIX_FILE\"\nexit 0\n")
    File.chmod(0o755, File.join(bin, "git"), File.join(bin, "docker"))
    env = {"PATH" => "#{bin}:#{ENV.fetch("PATH")}", "CI" => "true", "PROFILE" => profile,
           "FEATURES" => features, "IMAGE_TAG_PREFIX" => prefix, "TARGET_CACHE_ID" => "pr-42-#{sha}",
           "PREFIX_FILE" => File.join(dir, "prefix")}
    _stdout, stderr, status = Open3.capture3(env, "bash", bake, "forge-images", chdir: dir)
    assert status.success?, stderr
    File.read(File.join(dir, "prefix"))
  end

  # A10 classifies a local checkout-shaped step as PR-controlled by these exact input
  # names, so both actions that accept an exact PR SHA must keep them required.
  def assert_required_source_inputs(action)
    %w[source_repository source_sha].each do |name|
      assert_equal true, action.fetch("inputs").fetch(name).fetch("required"), name
    end
  end

  def git!(repo, *args)
    stdout, stderr, status = Open3.capture3("git", *args, chdir: repo)
    assert status.success?, stderr
    stdout
  end

  # Runs the shared runtime tag step. Returns [outputs, GITHUB_ENV lines], or nil on failure.
  def run_tag_step(dir, env, expect_success: true)
    tags = load_action("privileged-pr-runtime-setup").dig("runs", "steps").find { |step| step["id"] == "tags" }
    output = File.join(dir, "output")
    github_env = File.join(dir, "env")
    full_env = env.merge("PATH" => ENV.fetch("PATH"), "GITHUB_OUTPUT" => output, "GITHUB_ENV" => github_env,
                         "GITHUB_ACTION_PATH" => File.join(ROOT, ".github", "actions", "privileged-pr-runtime-setup"))
    _stdout, stderr, status = Open3.capture3(full_env, "bash", "-c", tags.fetch("run"))
    unless expect_success
      refute status.success?
      refute File.exist?(github_env)
      return nil
    end
    assert status.success?, stderr
    [File.read(output).lines(chomp: true).to_h { |line| line.split("=", 2) }, File.read(github_env).lines(chomp: true)]
  end

  def assert_registry_logins(action_steps, auth_id)
    logins = action_steps.select { |step| step["uses"].to_s.start_with?("docker/login-action@") }
    assert_equal REGISTRIES, logins.map { |step| step.dig("with", "registry") }
    auth_index = action_steps.index { |step| step["id"] == auth_id }
    logins.each do |login|
      assert_equal DOCKER_LOGIN_PIN, login.fetch("uses")
      assert_equal "oauth2accesstoken", login.dig("with", "username")
      assert_equal "${{ steps.#{auth_id}.outputs.access_token }}", login.dig("with", "password")
      assert_operator auth_index, :<, action_steps.index(login)
    end
  end

  # Asserts the initial checkout of the event base into trusted-base/.
  def assert_trusted_checkout(job)
    checkouts = checkout_steps(job)
    assert_operator checkouts.length, :>=, 1
    trusted = checkouts.first
    assert_equal CHECKOUT_PIN, trusted.fetch("uses")
    assert_equal "trusted-base", trusted.dig("with", "path")
    assert_equal "${{ github.repository }}", trusted.dig("with", "repository")
    assert_equal "${{ inputs.BASE_SHA }}", trusted.dig("with", "ref")
    assert_equal false, trusted.dig("with", "persist-credentials")
    steps(job).index(trusted)
  end
end
