# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

class ClaimNftButtonComponent < ViewComponent::Base
  include ActionText::Engine.helpers

  def initialize(nft_offer:, wallet:, **rest)
    @rest = rest
    @nft_offer = nft_offer
    @wallet = wallet

    @rest[:data] ||= {}
    @rest[:data][:controller] = 'claim-nft'
    @rest[:data][:action] = 'claim-nft#handleClick'
  end

  private

  # Enables use of form_with helper.
  def main_app
    Rails.application.class.routes.url_helpers
  end
end
