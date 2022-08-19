# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

ActiveAdmin.register NetworkOperation do
  permit_params :title, :content

  form do |f|
    f.semantic_errors
    f.inputs :title, :content
    f.actions
  end

  show do
    h3 network_operation.title
    div do
      sanitize network_operation.content
    end
  end

  sidebar :notifications, only: [:show] do
    ul do
      li do
        link_to 'Send governance proposal notification',
                notify_admin_network_operation_path(network_operation,
                                                    notification_type: :governance_proposal_notification)
      end
      li do
        link_to 'Send node upgrade notification',
                notify_admin_network_operation_path(network_operation, notification_type: :node_upgrade_notification)
      end
    end
  end

  member_action :notify, method: %i[get post] do
    notification_types = {
      governance_proposal_notification: GovernanceProposalNotification,
      node_upgrade_notification: NodeUpgradeNotification
    }.freeze

    @network_operation = resource
    unless @network_operation.notified_at.nil?
      return redirect_to admin_network_operation_path(@network_operation),
                         notice: "Notifications were already sent at #{@network_operation.notified_at.to_fs}."
    end

    @notification_type = params.require(:notification_type).to_sym
    unless notification_types.include?(@notification_type)
      return redirect_to admin_network_operation_path(@network_operation),
                         notice: "#{@notification_type} is not a valid notification type."
    end

    @users = User
             .left_outer_joins(:notification_preferences)
             .where(users: { notification_preferences: { delivery_method: :email,
                                                         @notification_type => true } })
             .or(User.where(users: { notification_preferences: { id: nil } }))

    if request.post?
      notification = notification_types[@notification_type].with(network_operation: @network_operation)
      notification.deliver_later(@users)
      @network_operation.touch(:notified_at)
      return redirect_to admin_network_operation_path(@network_operation),
                         notice: 'Notifications queued for delivery.'
    end
  end
end
