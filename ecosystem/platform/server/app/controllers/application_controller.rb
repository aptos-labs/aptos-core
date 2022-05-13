# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

require 'logging/logs'

class ApplicationController < ActionController::Base
  include Logging::Logs

  before_action :set_csrf_cookie
  before_action :set_logging_metadata
  before_action :set_sentry_metadata

  protect_from_forgery with: :exception

  def set_csrf_cookie
    cookies['CSRF-TOKEN'] = {
      value: form_authenticity_token,
      secure: true,
      same_site: :strict,
      domain: ENV.fetch('SITE_DOMAIN', 'localhost')
    }
  end

  def after_sign_in_path_for(user)
    stored_location = stored_location_for(user)
    return stored_location if stored_location.present?

    if user.email.nil?
      onboarding_email_path
    else
      it1_path
    end
  end

  def admin_access_denied(_exception)
    head :forbidden
  end

  def ensure_confirmed!
    redirect_to onboarding_email_path unless current_user&.confirmed?
  end

  def append_info_to_payload(payload)
    super
    # Add metadata to lograge request logs.
    payload[:request_id] = request.request_id
    payload[:user_id] = current_user&.id
  end

  def set_logging_metadata
    # Add metadata to thread local for Logging::Logs.log().
    Thread.current.thread_variable_set(REQUEST_ID_KEY, request.request_id)
    Thread.current.thread_variable_set(USER_ID_KEY, current_user&.id)
  end

  def set_sentry_metadata
    Sentry.set_user(id: current_user.id) if current_user
    Sentry.set_tags(request_id: request.request_id)
  end
end
