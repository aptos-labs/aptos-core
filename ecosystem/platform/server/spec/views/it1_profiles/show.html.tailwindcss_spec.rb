# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'it1_profiles/show', type: :view do
  before(:each) do
    @it1_profile = assign(:it1_profile, It1Profile.create!)
  end

  it 'renders attributes in <p>' do
    render
  end
end
