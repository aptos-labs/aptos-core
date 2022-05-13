# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

require 'MailchimpTransactional'

class SendEmailJobError < StandardError; end

BAD_STATUSES = %w[rejected invalid].freeze

class SendEmailJob < ApplicationJob
  # Ex args: { user_id: 32, template_name: 'some-template', template_vars: { SOME_MAILMERGE_KEY: 'My Value!' } }
  def perform(args)
    @args = args
    @user = User.find(args[:user_id])
    sentry_scope.set_user(id: @user.id)

    sentry_scope.set_context(:job_args, { template_name: })
    sentry_scope.set_context(:job_args, { template_vars: })

    send_email
  end

  # https://mailchimp.com/developer/transactional/api/messages/send-using-message-template/
  def send_email
    client = MailchimpTransactional::Client.new(ENV.fetch('MAILCHIMP_API_KEY', nil))
    body = email_body
    sentry_scope.set_context(:email_body, body)
    results = client.messages.send_template(body)
    handle_results(results)
  rescue StandardError => e
    Sentry.capture_exception(e)
    raise SendEmailJobError
  end

  def email_body
    {
      template_name:,
      template_content: [{}],
      message:
    }
  end

  private

  def handle_results(results)
    # [{"email"=>"josh@somedomain.com", "status"=>"sent", "_id"=>"some-id", "reject_reason"=>nil}, ...]
    results.each do |res|
      # the sending status of the recipient. Possible values: "sent", "queued", "rejected", or "invalid".
      raise SendEmailJobError, "Could not send email: #{res['reject_reason']}" if BAD_STATUSES.include? res['status']
    rescue StandardError => e
      Rails.logger.warn e
      Sentry.capture_exception(e)
    end
  end

  # Override any of these to have much nicer email job classes!
  def template_name
    @args[:template_name]
  end

  def template_vars
    @args[:template_vars] || {}
  end

  def message
    {
      global_merge_vars:,
      from_email:,
      to: to_emails.map { |em| { email: em } }
    }
  end

  def to_emails
    [@user.email]
  end

  def from_email
    'community@aptoslabs.com'
  end

  def global_merge_vars
    template_vars.map { |k, v| { name: k, content: v } }
  end
end
