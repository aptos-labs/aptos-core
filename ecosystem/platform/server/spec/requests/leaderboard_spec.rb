# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0
require 'rails_helper'

RSpec.describe 'Leaderboards', type: :request do
  describe 'GET /it1' do
    it 'returns http success' do
      get '/leaderboard/it1'
      expect(response).to have_http_status(:success)
    end
  end
end
