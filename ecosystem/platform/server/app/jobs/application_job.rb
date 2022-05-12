# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

class ApplicationJob < ActiveJob::Base
  # Automatically retry jobs that encountered a deadlock
  retry_on ActiveRecord::Deadlocked

  # Add context to sentry for jobs
  around_perform do |job, block|
    Sentry.configure_scope do |scope|
      scope.set_context(:job_args, job.arguments.first)
      scope.set_tags(job_name: job.class.name)
      scope.set_user(id: job.arguments.first[:user_id]) if job.arguments.first.try(:include?, :user_id)
      job.sentry_scope = scope
      block.call
    end
  end

  # @return [Sentry::Scope]
  attr_accessor :sentry_scope

  # Most jobs are safe to ignore if the underlying records are no longer available
  # discard_on ActiveJob::DeserializationError
end
