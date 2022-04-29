# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'it1_profiles/index', type: :view do
  before(:each) do
    assign(:it1_profiles, [
             It1Profile.create!,
             It1Profile.create!
           ])
  end

  it 'renders a list of it1_profiles' do
    render
  end
end
