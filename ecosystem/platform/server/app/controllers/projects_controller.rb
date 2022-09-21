# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

class ProjectsController < ApplicationController
  before_action :ensure_projects_enabled!
  before_action :authenticate_user!, except: %i[index show]
  before_action :ensure_confirmed!, except: %i[index show]
  respond_to :html

  # GET /projects
  def index
    @categories = Category.all.index_by(&:id)
    @projects = params[:s].blank? ? Project : Project.search(params[:s])
    @projects = @projects.where(public: true, verified: true)
                         .includes(:project_categories)
                         .with_attached_thumbnail

    selected_category = params[:category].to_i
    selected_category = nil if params[:category].blank?
    @projects = @projects.filter_by_category(selected_category) if selected_category

    @groups = @projects.each_with_object({}) do |project, groups|
      project.project_categories.each do |project_category|
        (groups[project_category.category_id] ||= []) << project
      end
    end
    @groups.delete_if { |category_id| category_id != selected_category } if selected_category
  end

  # GET /projects/1
  def show
    @project = Project.find(params[:id])

    if @project.user_id != current_user&.id && !current_user&.is_root
      return head :not_found unless @project.verified
      return head :forbidden unless @project.public
    end
  end

  # GET /projects/new
  def new
    @project = Project.new
    @project.project_categories.new
    @project.project_members.new
  end

  # GET /projects/1/edit
  def edit
    @project = Project.find(params[:id])
    head :forbidden unless @project.user_id == current_user.id
  end

  # POST /projects
  def create
    params = project_params
    params[:user] = current_user
    @project = Project.new(params)

    return unless check_recaptcha

    if @project.save
      redirect_to project_url(@project),
                  notice: 'Project was successfully created. ' \
                          'It will not be publicly visible until approved by a moderator.'
    else
      render :new, status: :unprocessable_entity
    end
  end

  # PATCH/PUT /projects/1
  def update
    return unless check_recaptcha

    @project = Project.find(params[:id])
    return head :forbidden unless @project.user_id == current_user.id

    if @project.update(project_params)
      redirect_to project_url(@project), notice: 'Project was successfully updated.'
    else
      render :edit, status: :unprocessable_entity
    end
  end

  # DELETE /projects/1
  def destroy
    @project = Project.find(params[:id])
    return head :forbidden unless @project.user_id == current_user.id

    @project.destroy

    redirect_to ecosystem_url, notice: 'Project was successfully destroyed.'
  end

  private

  # Only allow a list of trusted parameters through.
  def project_params
    result = params.require(:project).permit(:title, :short_description, :full_description, :website_url, :github_url,
                                             :discord_url, :twitter_url, :telegram_url, :linkedin_url, :thumbnail,
                                             :youtube_url, :public,
                                             category_ids: [],
                                             project_members_attributes: %i[id user_id role public],
                                             screenshots: [])

    result.each_key do |key|
      result[key] = nil if key.to_s.ends_with?('_url') && result[key].blank?
    end

    result
  end

  def check_recaptcha
    recaptcha_v3_success = verify_recaptcha(action: 'projects/update', minimum_score: 0.5,
                                            secret_key: ENV.fetch('RECAPTCHA_V3_SECRET_KEY', nil), model: @project)
    recaptcha_v2_success = verify_recaptcha(model: @project) unless recaptcha_v3_success
    unless recaptcha_v3_success || recaptcha_v2_success
      @show_recaptcha_v2 = true
      respond_with(@project, status: :unprocessable_entity)
      return false
    end
    true
  end

  def ensure_projects_enabled!
    redirect_to root_path unless Flipper.enabled?(:projects)
  end
end
