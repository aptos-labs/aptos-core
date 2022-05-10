# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

class ApplicationController < ActionController::Base
  before_action :set_csrf_cookie

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
      overview_index_path
    end
  end

  def admin_access_denied(_exception)
    head :forbidden
  end

  def ensure_confirmed!
    redirect_to onboarding_email_path unless current_user.confirmed?
  end
end
