# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

class BaseNotification < Noticed::Base
  def self.deliver_by(delivery_method, options = {})
    notification_name = name.underscore
    guard_method_name = "deliver_#{delivery_method}_#{notification_name}?".to_sym

    unless respond_to?(guard_method_name)
      define_method guard_method_name do
        prefs = recipient.notification_preferences.where(delivery_method:).first
        return true if prefs.nil?

        prefs[notification_name]
      end
    end

    options[:if] = guard_method_name
    super(delivery_method, options)
  end
end
