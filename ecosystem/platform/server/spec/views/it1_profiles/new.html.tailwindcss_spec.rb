# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'it1_profiles/new', type: :view do
  before(:each) do
    assign(:it1_profile, It1Profile.new)
  end

  it 'renders new it1_profile form' do
    render

    assert_select 'form[action=?][method=?]', it1_profiles_path, 'post' do
    end
  end
end
