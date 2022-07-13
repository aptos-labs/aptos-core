require 'rails_helper'

RSpec.describe "proposals/show", type: :view do
  before(:each) do
    @proposal = assign(:proposal, Proposal.create!())
  end

  it "renders attributes in <p>" do
    render
  end
end
