# frozen_string_literal: true

module Api
  class UsersController < ::ApiController
    before_action :authenticate_user!

    def me
      @user = current_user
      render 'api/users/show.json.jbuilder'
    end

    def show
      @user = User.includes(:authorizations).find(params[:id])
      render 'api/users/show.json.jbuilder'
    end

    def update
      user_params = params.permit(:username, :is_developer, :is_node_operator, :mainnet_address)
      @user = User.includes(:authorizations).find(params[:id])
      # Only let users change their own profile
      head :forbidden unless @user.id == current_user.id
      @user.update!(user_params)
      render 'api/users/show.json.jbuilder'
    end
  end
end
