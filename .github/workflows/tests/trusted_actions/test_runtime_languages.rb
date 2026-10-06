# frozen_string_literal: true

require "minitest/autorun"
require_relative "../workflow_test_helper"

# Keep executable helpers under .github/actions and .github/ci on a small,
# explicit runtime surface.
class RuntimeLanguagesTest < Minitest::Test
  include WorkflowTestHelper

  JAVASCRIPT = /\.(?:c|m)?[jt]sx?\z/
  SHELL_WRAPPERS = %w[
    .github/actions/checkout-exact-pr-source/verify-source.sh
    .github/actions/install-grpcurl/install_grpcurl.sh
  ].freeze

  def tracked_files
    Dir.chdir(ROOT) do
      IO.popen(["git", "ls-files", "--", ".github/actions", ".github/ci"], &:readlines).map(&:chomp)
    end
  end

  def test_no_javascript_runtime_in_actions_or_ci
    assert_empty tracked_files.grep(JAVASCRIPT)
  end

  def test_shell_scripts_are_only_the_known_tool_wrappers
    assert_equal SHELL_WRAPPERS.sort, tracked_files.grep(/\.sh\z/).sort
  end

  # Older action files are outside the policy scope and may not pass
  # SafeYaml, so this reads only the `using:` line.
  def test_every_action_is_composite
    Dir.glob(File.join(ROOT, ".github/actions/*/action.{yml,yaml}")).each do |path|
      assert_equal "composite", File.read(path)[/^\s*using:\s*["']?([\w-]+)/, 1], path
    end
  end
end
