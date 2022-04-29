# frozen_string_literal: true

class It1ProfilesController < ApplicationController
  before_action :authenticate_user!
  before_action :set_it1_profile, only: %i[show edit update destroy]

  # GET /it1_profiles/new
  def new
    @it1_profile = It1Profile.new
  end

  # GET /it1_profiles/1/edit
  def edit; end

  # POST /it1_profiles or /it1_profiles.json
  def create
    params = it1_profile_params
    params[:user] = current_user
    @it1_profile = It1Profile.new(params)

    respond_to do |format|
      if @it1_profile.save
        format.html { redirect_to overview_index_path, notice: 'IT1 application was successfully created.' }
      else
        format.html { render :new, status: :unprocessable_entity }
      end
    end
  end

  # PATCH/PUT /it1_profiles/1 or /it1_profiles/1.json
  def update
    respond_to do |format|
      if @it1_profile.update(it1_profile_params)
        format.html { redirect_to overview_index_path, notice: 'IT1 application was successfully updated.' }
      else
        format.html { render :edit, status: :unprocessable_entity }
      end
    end
  end

  private

  # Use callbacks to share common setup or constraints between actions.
  def set_it1_profile
    @it1_profile = It1Profile.find(params[:id])
    head :forbidden unless @it1_profile.user_id == current_user.id
  end

  # Only allow a list of trusted parameters through.
  def it1_profile_params
    params.fetch(:it1_profile, {}).permit(:consensus_key, :account_key, :network_key, :validator_address,
                                          :validator_port, :metrics_port, :fullnode_address, :fullnode_port, :terms)
  end
end
