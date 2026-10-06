require "minitest/autorun"
require_relative "../workflow_test_helper"

# Every action that runs trusted Python must be a thin composite wrapper:
# inputs reach Python only through env, and the launcher runs with -I.
class PythonActionWrapperTests < Minitest::Test
  include WorkflowTestHelper

  LAUNCHER = /\Apython3 -I "\$GITHUB_ACTION_PATH\/\.\.\/\.\.\/ci\/run_action\.py" [a-z-]+\z/

  def python_actions
    Dir.glob(File.join(ROOT, ".github/actions/*/action.yml")).filter_map do |path|
      dir = File.basename(File.dirname(path))
      action = load_action(dir)
      steps = action.fetch("runs").fetch("steps", [])
      [dir, action] if steps.any? { |step| step.fetch("run", "").include?("run_action.py") }
    end
  end

  def test_python_actions_are_composite_wrappers_around_the_launcher
    actions = python_actions
    assert_includes actions.map(&:first), "compute-authorized"
    actions.each do |dir, action|
      action.fetch("runs").fetch("steps").select { |step| step.key?("run") }.each do |step|
        assert_match LAUNCHER, step.fetch("run").strip, dir
        assert_equal "bash", step.fetch("shell"), dir
      end
    end
  end
end
