# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'it1_profiles/edit', type: :view do
  before(:each) do
    @it1_profile = assign(:it1_profile, It1Profile.create!)
  end

  it 'renders the edit it1_profile form' do
    render

    assert_select 'form[action=?][method=?]', it1_profile_path(@it1_profile), 'post' do
    end
  end
end
