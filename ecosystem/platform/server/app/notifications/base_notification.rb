# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

class BaseNotification < Noticed::Base
  def self.deliver_by(delivery_method, options = {})
    notification_name = name.underscore
    delivery_method_guard_name = "deliver_#{delivery_method}?".to_sym
    preference_guard_name = "deliver_#{delivery_method}_#{notification_name}?".to_sym

    unless respond_to?(preference_guard_name)
      define_method preference_guard_name do
        # First, check generic delivery_method logic.
        return false if respond_to?(delivery_method_guard_name) && !send(delivery_method_guard_name)

        # Next, check for the delivery_method/notification_name preference.
        prefs = recipient.notification_preferences.where(delivery_method:).first
        return false if prefs.nil?

        prefs[notification_name]
      end
    end

    options[:if] = preference_guard_name
    super(delivery_method, options)
  end

  def deliver_email?
    !recipient.email.nil?
  end
end
