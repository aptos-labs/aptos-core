# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Onboardings', type: :request do
  describe 'GET /email' do
    it 'returns http success' do
      get '/onboarding/email'
      expect(response).to have_http_status(:success)
    end
  end

  describe 'GET /roles' do
    it 'returns http success' do
      get '/onboarding/roles'
      expect(response).to have_http_status(:success)
    end
  end
end
