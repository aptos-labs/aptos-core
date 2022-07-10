# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

class It2ProfilesController < ApplicationController
  before_action :authenticate_user!
  before_action :ensure_registration_enabled!
  before_action :ensure_confirmed!
  before_action :set_it2_profile, only: %i[show edit update destroy]
  respond_to :html

  def show
    redirect_to edit_it2_profile_path(params.fetch(:id))
  end

  # GET /it2_profiles/new
  def new
    redirect_to edit_it2_profile_path(current_user.it2_profile) if current_user.it2_profile.present?
    @it2_profile = It2Profile.new
  end

  # GET /it2_profiles/1/edit
  def edit; end

  # POST /it2_profiles or /it2_profiles.json
  def create
    params = it2_profile_params
    params[:user] = current_user
    @it2_profile = It2Profile.new(params)

    return unless check_recaptcha

    v = NodeHelper::IPResolver.new(@it2_profile.validator_address)

    if v.ip.ok
      @it2_profile.validator_ip = v.ip.ip
    else
      @it2_profile.errors.add :validator_address, v.ip.message
      respond_with(@it2_profile, status: :unprocessable_entity) and return
    end

    if @it2_profile.save
      log @it2_profile, 'created'
      @it2_profile.enqueue_nhc_job(true)
      redirect_to it2_path,
                  notice: 'AIT2 node information updated, running node health checker' and return
    end
    respond_with(@it2_profile, status: :unprocessable_entity)
  end

  # PATCH/PUT /it2_profiles/1 or /it2_profiles/1.json
  def update
    v = NodeHelper::IPResolver.new(it2_profile_params[:validator_address])

    return unless check_recaptcha

    if v.ip.ok
      @it2_profile.validator_ip = v.ip.ip
    else
      @it2_profile.errors.add :validator_address, v.ip.message
      respond_with(@it2_profile, status: :unprocessable_entity) and return
    end

    ip_changed = @it2_profile.validator_ip_changed?
    @it2_profile.maybe_set_validated_to_false
    needs_revalidation = @it2_profile.needs_revalidation?
    if @it2_profile.update(it2_profile_params)
      log @it2_profile, 'updated'
      if needs_revalidation || !@it2_profile.validator_verified?
        @it2_profile.enqueue_nhc_job(ip_changed)
        redirect_to it2_path,
                    notice: 'AIT2 node information updated, running node health checker' and return
      end

      if @it2_profile.validator_verified?
        redirect_to it2_path,
                    notice: 'AIT2 node information updated, validator is verified' and return
      end

    end
    respond_with(@it2_profile, status: :unprocessable_entity)
  end

  private

  def check_recaptcha
    recaptcha_v3_success = verify_recaptcha(action: 'it2/update', minimum_score: 0.5,
                                            secret_key: ENV.fetch('RECAPTCHA_V3_SECRET_KEY', nil), model: @it2_profile)
    recaptcha_v2_success = verify_recaptcha(model: @it2_profile) unless recaptcha_v3_success
    unless recaptcha_v3_success || recaptcha_v2_success
      @show_recaptcha_v2 = true
      respond_with(@it2_profile, status: :unprocessable_entity)
      return false
    end
    true
  end

  # Use callbacks to share common setup or constraints between actions.
  def set_it2_profile
    @it2_profile = It2Profile.find(params[:id])
    head :forbidden unless @it2_profile.user_id == current_user.id
  end

  # Only allow a list of trusted parameters through.
  def it2_profile_params
    params.fetch(:it2_profile, {}).permit(:consensus_key, :account_key, :network_key, :validator_address,
                                          :validator_port, :validator_api_port, :validator_metrics_port,
                                          :fullnode_address, :fullnode_port, :fullnode_network_key, :terms_accepted)
  end

  def ensure_registration_enabled!
    redirect_to root_path unless Flipper.enabled?(:it2_registration_open)
    redirect_to it2_path if Flipper.enabled?(:it2_node_registration_disabled,
                                             current_user) || Flipper.enabled?(:it2_registration_closed, current_user)
  end
end
