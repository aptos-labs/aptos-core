# frozen_string_literal: true

require "minitest/autorun"
require_relative "../workflow_test_helper"

class DockerForgePrReportWorkflowTests < Minitest::Test
  H = WorkflowTestHelper

  def setup
    @workflow = H.load_workflow("docker-forge-pr-report.yaml")
    @prepare = H.jobs(@workflow).fetch("prepare_report")
    @publish = H.jobs(@workflow).fetch("publish_reports")
  end

  def test_workflow_run_preparation_is_base_owned_and_read_only
    triggers = @workflow.fetch("on")
    assert_equal ["workflow_run"], triggers.keys
    assert_equal [H.load_workflow("docker-build-test.yaml").fetch("name")], triggers.dig("workflow_run", "workflows")
    assert_equal ["completed"], triggers.dig("workflow_run", "types")
    assert_equal({ "actions" => "read", "contents" => "read" }, @workflow.fetch("permissions"))
    assert_includes @workflow.fetch("concurrency").fetch("group"), "github.event.workflow_run.id"

    assert_equal "${{ github.event.workflow_run.event == 'pull_request_target' }}", @prepare.fetch("if")
    assert_equal({ "actions" => "read", "contents" => "read" }, @prepare.fetch("permissions"))
    checkouts = @prepare.fetch("steps").select { |step| step["uses"].to_s.start_with?("actions/checkout@") }
    assert_equal 1, checkouts.length
    assert_equal({ "repository" => "${{ github.repository }}", "ref" => "${{ github.sha }}", "persist-credentials" => false },
                 checkouts.first.fetch("with").slice("repository", "ref", "persist-credentials"))
    source = @prepare.to_s
    %w[github.event.workflow_run.head_sha github.event.workflow_run.head_repository download-artifact
       upload-artifact github.event.pull_request].each { |text| refute_includes source, text }
  end

  def test_reporter_step_passes_only_the_run_id_and_exposes_every_action_output
    report = @prepare.fetch("steps").find { |step| step["uses"] == "./.github/actions/docker-forge-pr-report" }
    refute_nil report
    assert_equal({ "run_id" => "${{ github.event.workflow_run.id }}" }, report.fetch("with"))
    refute report.key?("env")
    outputs = H.load_action("docker-forge-pr-report").fetch("outputs").keys
    assert_equal outputs.sort, @prepare.fetch("outputs").keys.sort
    outputs.each do |name|
      assert_equal "${{ steps.#{report.fetch("id")}.outputs.#{name} }}", @prepare.dig("outputs", name)
    end
  end

  def test_reporter_publishes_one_pinned_sticky_comment_per_matrix_record
    assert_equal ["prepare_report"], @publish.fetch("needs")
    assert_equal "${{ needs.prepare_report.outputs.report_matrix != '{\"include\":[]}' }}", @publish.fetch("if")
    assert_equal({ "contents" => "read", "pull-requests" => "write" }, @publish.fetch("permissions"))
    assert_equal({ "max-parallel" => 1, "matrix" => "${{ fromJSON(needs.prepare_report.outputs.report_matrix) }}" },
                 @publish.fetch("strategy"))

    assert_equal 1, @publish.fetch("steps").length
    step = @publish.fetch("steps").first
    assert_equal "marocchino/sticky-pull-request-comment@39c5b5dc7717447d0cba270cd115037d32d28443", step.fetch("uses")
    assert_equal({ "number" => "${{ needs.prepare_report.outputs.pr_number }}", "header" => "${{ matrix.header }}",
                   "hide_and_recreate" => true, "hide_classify" => "OUTDATED" },
                 step.fetch("with").slice("number", "header", "hide_and_recreate", "hide_classify"))
    message = step.dig("with", "message")
    %w[matrix.title matrix.result needs.prepare_report.outputs.head_sha needs.prepare_report.outputs.run_url].each do |value|
      assert_includes message, "${{ #{value} }}"
    end
    refute_includes message, "github.event.workflow_run"
  end
end
