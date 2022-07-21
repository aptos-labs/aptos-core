# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

class DiscourseJob < ApplicationJob
  memoize def discourse_user_id
    return @user.discourse_id if @user.discourse_id.present?

    id = @client.by_external_id(@user.external_id)['id']
    Rails.logger.debug("Fetched forum user id #{id} for user #{@user.id} - #{@user.external_id}")
    if id.present?
      @user.discourse_id = id
      @user.save
    end
    id
  rescue DiscourseApi::NotFoundError
    Rails.logger.debug("Forum account does not exist yet for user #{@user.id} - #{@user.external_id}")
  end
end
