# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

class It2SurveysController < ApplicationController
  before_action :authenticate_user!
  before_action :ensure_confirmed!
  before_action :set_it2_survey, only: %i[show edit update destroy]
  layout 'it2'
  respond_to :html

  def show
    redirect_to edit_it2_survey_path(params.fetch(:id))
  end

  # GET /it2_surveys/new
  def new
    redirect_to edit_it2_survey_path(current_user.it2_survey) if current_user.it2_survey.present?
    @it2_survey = It2Survey.new
  end

  # GET /it2_surveys/1/edit
  def edit; end

  # POST /it2_surveys
  def create
    params = it2_survey_params
    params[:user] = current_user
    @it2_survey = It2Survey.new(params)

    if @it2_survey.save
      log @it2_survey, 'created'
      redirect_to it2_path, notice: 'Your survey response has been recorded.'
    else
      respond_with(@it2_survey, status: :unprocessable_entity)
    end
  end

  # PATCH/PUT /it2_surveys/1
  def update
    if @it2_survey.update(it2_survey_params)
      log @it2_survey, 'updated'
      redirect_to it2_path, notice: 'Survey response updated.'
    else
      respond_with(@it2_survey, status: :unprocessable_entity)
    end
  end

  private

  def set_it2_survey
    @it2_survey = It2Survey.find(params[:id])
    head :forbidden unless @it2_survey.user_id == current_user.id
  end

  def it2_survey_params
    params.require(:it2_survey).permit(:user_id, :persona, :participate_reason, :qualified_reason, :website,
                                       :interest_reason)
  end
end
