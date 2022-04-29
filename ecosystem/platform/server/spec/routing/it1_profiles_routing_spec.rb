# frozen_string_literal: true

require 'rails_helper'

RSpec.describe It1ProfilesController, type: :routing do
  describe 'routing' do
    it 'routes to #index' do
      expect(get: '/it1_profiles').to route_to('it1_profiles#index')
    end

    it 'routes to #new' do
      expect(get: '/it1_profiles/new').to route_to('it1_profiles#new')
    end

    it 'routes to #show' do
      expect(get: '/it1_profiles/1').to route_to('it1_profiles#show', id: '1')
    end

    it 'routes to #edit' do
      expect(get: '/it1_profiles/1/edit').to route_to('it1_profiles#edit', id: '1')
    end

    it 'routes to #create' do
      expect(post: '/it1_profiles').to route_to('it1_profiles#create')
    end

    it 'routes to #update via PUT' do
      expect(put: '/it1_profiles/1').to route_to('it1_profiles#update', id: '1')
    end

    it 'routes to #update via PATCH' do
      expect(patch: '/it1_profiles/1').to route_to('it1_profiles#update', id: '1')
    end

    it 'routes to #destroy' do
      expect(delete: '/it1_profiles/1').to route_to('it1_profiles#destroy', id: '1')
    end
  end
end
