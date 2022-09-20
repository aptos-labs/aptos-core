# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

require 'discourse_api'

class DiscourseController < ApplicationController
  before_action :set_post_login_redirect!
  before_action :authenticate_user!
  before_action :ensure_confirmed!

  def sso
    query_str = cookies['FORUM-SSO'] || request.query_string

    # This allows hitting this sso for the first time from our side, we redirect to forum, which redirects back to us
    # with the nonce, which we read, and then redirect back to the forum, with the user logged in there at this point.
    # This is intended for when a user is already logged in, for a seamless SSO!
    if query_str.blank?
      redirect_to DiscourseHelper.discourse_url('/session/sso?return_path=%2F'),
                  allow_other_host: true
      return
    end

    sso = DiscourseApi::SingleSignOn.parse(query_str, DiscourseHelper.sso_secret)
    cookies.delete 'FORUM-SSO'

    sso.email = current_user.email
    sso.username = current_user.username
    sso.external_id = current_user.external_id # unique id for each user of your application
    sso.sso_secret = DiscourseHelper.sso_secret

    sso.admin = current_user.is_root?

    add_groups = []
    sso.add_groups = add_groups if add_groups.present?

    redirect_to sso.to_url(DiscourseHelper.discourse_url('/session/sso_login')), allow_other_host: true
  end

  protected

  def set_post_login_redirect!
    cookies['FORUM-SSO'] = {
      value: request.query_string,
      httponly: true
    }
  end
end
