# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

require 'test_helper'

class NftOffersControllerTest < ActionDispatch::IntegrationTest
  include Devise::Test::IntegrationHelpers

  test 'claiming an nft with unconfirmed email' do
    Flipper.enable(:require_email_verification_for_nft)
    user = FactoryBot.create(:user, email: nil, unconfirmed_email: Faker::Internet.email)
    wallet = FactoryBot.create(:wallet, user:)
    sign_in user

    put nft_offer_path(slug: 'aptos-zero'), params: {
      wallet: wallet.public_key,
      wallet_name: wallet.wallet_name
    }

    assert_response :see_other
    assert_redirected_to onboarding_email_path
  end
end
