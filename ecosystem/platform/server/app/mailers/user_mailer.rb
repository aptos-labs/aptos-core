# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

class UserMailer < ApplicationMailer
  def node_maintenance_notification
    @user = params[:recipient]
    @node_maintenance = params[:node_maintenance]
    mail(to: @user.email, subject: @node_maintenance.title)
  end
end
