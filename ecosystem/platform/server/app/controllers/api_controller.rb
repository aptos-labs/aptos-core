# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

class ApiController < ActionController::API
  include AbstractController::Translation
  include ActionController::Cookies

  before_action :set_default_response_format

  def set_default_response_format
    request.format = :json
  end
end
