require "rails_helper"

RSpec.describe ProposalsController, type: :routing do
  describe "routing" do
    it "routes to #index" do
      expect(get: "/proposals").to route_to("proposals#index")
    end

    it "routes to #new" do
      expect(get: "/proposals/new").to route_to("proposals#new")
    end

    it "routes to #show" do
      expect(get: "/proposals/1").to route_to("proposals#show", id: "1")
    end

    it "routes to #edit" do
      expect(get: "/proposals/1/edit").to route_to("proposals#edit", id: "1")
    end


    it "routes to #create" do
      expect(post: "/proposals").to route_to("proposals#create")
    end

    it "routes to #update via PUT" do
      expect(put: "/proposals/1").to route_to("proposals#update", id: "1")
    end

    it "routes to #update via PATCH" do
      expect(patch: "/proposals/1").to route_to("proposals#update", id: "1")
    end

    it "routes to #destroy" do
      expect(delete: "/proposals/1").to route_to("proposals#destroy", id: "1")
    end
  end
end
