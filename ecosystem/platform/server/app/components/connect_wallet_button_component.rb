# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

class ConnectWalletButtonComponent < ViewComponent::Base
  include ActionText::Engine.helpers

  def initialize(wallet:, required_network: nil, **rest)
    @rest = rest
    @wallet = wallet
    @required_network = required_network
    @turbo_frame = @rest[:turbo_frame]
    @dialog = DialogComponent.new
  end

  def supported_wallets
    %w[petra martian]
  end

  private

  # Enables use of form_with helper.
  def main_app
    Rails.application.class.routes.url_helpers
  end
end
