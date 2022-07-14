require 'rails_helper'

RSpec.describe "proposals/index", type: :view do
  before(:each) do
    assign(:proposals, [
      Proposal.create!(),
      Proposal.create!()
    ])
  end

  it "renders a list of proposals" do
    render
  end
end
