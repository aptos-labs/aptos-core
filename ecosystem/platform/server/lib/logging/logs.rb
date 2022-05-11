# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

module Logging
  module Logs
    REQUEST_ID_KEY = 'request_id'
    USER_ID_KEY = 'user_id'

    def log(message_or_object, message = nil)
      result = {}

      result[:time] = Time.now.to_f
      result[:class] = self.class

      request_id = Thread.current.thread_variable_get(REQUEST_ID_KEY)
      result[:request_id] = request_id

      user_id = Thread.current.thread_variable_get(USER_ID_KEY)
      result[:user_id] = user_id if user_id.present?

      if message.nil?
        message = message_or_object
      else
        object = message_or_object
        result[:object_class] = object.class
        result[:object_id] = object.id if object.respond_to?(:id)
      end

      result[:message] = message

      Rails.logger.info(result.to_json)
    end
  end
end
