# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

class DiscourseSyncSsoJob < DiscourseJob
  # Ex args: { user_id: 32 }
  def perform(args)
    @args = args
    @user = User.find(args[:user_id])

    unless @user.registration_completed?
      return Rails.logger.debug("User not confirmed: #{@user.id} - #{@user.external_id}")
    end

    @client = DiscourseHelper.system_client

    sync_sso
  end

  def sync_sso
    return if discourse_user_id.nil?

    res = @client.sync_sso(
      sso_secret: DiscourseHelper.sso_secret,
      username: @user.username,
      email: @user.email,
      external_id: @user.external_id,
      admin: @user.is_root?
    )
    Rails.logger.debug("User SSO synced to discourse: #{@user.id} - #{@user.external_id}")
    res
  end
end

# DiscourseSyncSsoJob.perform_now({ user_id: 5 })
