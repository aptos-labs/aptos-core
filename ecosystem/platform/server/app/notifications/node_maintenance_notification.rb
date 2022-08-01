# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

# To deliver this notification:
#
# NodeMaintenanceNotification.with(node_maintenance: @node_maintenance).deliver_later(current_user)
# NodeMaintenanceNotification.with(node_maintenance: @node_maintenance).deliver(current_user)

class NodeMaintenanceNotification < Noticed::Base
  # Add your delivery methods
  #
  deliver_by :database
  deliver_by :email, mailer: 'UserMailer'
  # deliver_by :slack
  # deliver_by :custom, class: "MyDeliveryMethod"

  # Add required params
  #
  param :node_maintenance

  # Define helper methods to make rendering easier.
  def url
    node_maintenance_path(params[:node_maintenance])
  end
end
