require 'rails_helper'

RSpec.describe "proposals/edit", type: :view do
  before(:each) do
    @proposal = assign(:proposal, Proposal.create!())
  end

  it "renders the edit proposal form" do
    render

    assert_select "form[action=?][method=?]", proposal_path(@proposal), "post" do
    end
  end
end
