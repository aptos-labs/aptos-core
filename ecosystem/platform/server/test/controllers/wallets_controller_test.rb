# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

require 'test_helper'

class WalletsControllerTest < ActionDispatch::IntegrationTest
  include Devise::Test::IntegrationHelpers

  test 'create new wallet' do
    user = FactoryBot.create(:user)
    sign_in user

    assert_difference('Wallet.count') do
      post wallets_path, params: {
        wallet: {
          network: 'ait3',
          wallet_name: 'petra',
          public_key: '0x59506fcdc1f45c2f289bfd0f240c75995af54c31c5ed796b318d780b340471f6',
          challenge: '999999999999999999999999',
          signed_challenge: '0x7b7ded9a874ea2850528c9d5690a81bddde8064b446885b22bc68a2553320ee1854cca3d52c1f9fc8135e6' \
                            '073784164d9c07bf8437a5850787e46729ad878807'
        }
      }
    end
  end
end
