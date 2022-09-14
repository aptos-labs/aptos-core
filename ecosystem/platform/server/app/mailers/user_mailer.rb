# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

class UserMailer < ApplicationMailer
  layout 'user_mailer'

  def node_upgrade_notification
    @user = params[:recipient]
    @network_operation = params[:network_operation]
    mail(to: @user.email, subject: @network_operation.title)
  end

  def governance_proposal_notification
    @user = params[:recipient]
    @network_operation = params[:network_operation]
    mail(to: @user.email, subject: @network_operation.title)
  end
end
