# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

class ConnectWalletButtonComponent < ViewComponent::Base
  include ActionText::Engine.helpers
  include Turbo::FramesHelper

  def initialize(wallet:, **rest)
    @rest = rest
    @wallet = wallet
    @turbo_frame = @rest[:turbo_frame]
  end

  private

  # Enables use of form_with helper.
  def main_app
    Rails.application.class.routes.url_helpers
  end
end
