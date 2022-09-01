# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

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
          public_key: '0x7b9bcc8610e7cc121de936ba00e214c02ff4b5cfdb3fcfb2267959a941ecc521'
        },
        challenge: '999999999999999999999999',
        signed_challenge: 'b843bfce9bc811dc23da5c2d9cdca7b5b59042e060b91c1dc3534cefa2c2058d15615e8e2866888eb65415203' \
                          '222444634462e50ffd8f24fe0387859946b3209'
      }
      assert @response.parsed_body['created']
    end
  end
end
