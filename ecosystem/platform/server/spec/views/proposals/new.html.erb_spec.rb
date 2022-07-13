require 'rails_helper'

RSpec.describe "proposals/new", type: :view do
  before(:each) do
    assign(:proposal, Proposal.new())
  end

  it "renders new proposal form" do
    render

    assert_select "form[action=?][method=?]", proposals_path, "post" do
    end
  end
end
