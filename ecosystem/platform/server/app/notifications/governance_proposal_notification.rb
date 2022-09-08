# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

# To deliver this notification:
#
# GovernanceProposalNotification.with(network_operation: @network_operation).deliver_later(current_user)
# GovernanceProposalNotification.with(network_operation: @network_operation).deliver(current_user)

class GovernanceProposalNotification < BaseNotification
  # Add your delivery methods
  #
  deliver_by :database
  deliver_by :email, mailer: 'UserMailer'
  # deliver_by :slack
  # deliver_by :custom, class: "MyDeliveryMethod"

  # Add required params
  #
  param :network_operation

  # Define helper methods to make rendering easier.
  def url
    network_operation_path(params[:network_operation])
  end
end
