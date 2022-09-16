# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

class ClaimNftButtonComponent < ViewComponent::Base
  include ActionText::Engine.helpers

  def initialize(nft_offer:, wallet:, recaptcha_v2: false, **rest)
    @rest = rest
    @nft_offer = nft_offer
    @wallet = wallet
    @recaptcha_v2 = recaptcha_v2

    @rest[:data] ||= {}
    @rest[:data][:controller] = 'claim-nft'
    @rest[:data][:claim_nft_address_value] = @wallet.address
    @rest[:data][:claim_nft_network_value] = @wallet.network
    @rest[:data][:claim_nft_api_url_value] = @wallet.api_url
    @rest[:data][:claim_nft_module_address_value] = @nft_offer.module_address
  end

  private

  # Enables use of form_with helper.
  def main_app
    Rails.application.class.routes.url_helpers
  end
end
