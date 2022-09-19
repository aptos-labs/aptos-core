# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

ActiveAdmin.register Project do
  actions :all

  permit_params :verified
  includes :user

  index do
    selectable_column
    id_column
    column :user
    column :title
    column :short_description
    column :website_url
    column :verified
    actions
  end

  filter :title
  filter :short_description
  filter :website_url
  filter :verified

  show do
    default_main_content
    attributes_table do
      row :thumbnail do
        image_tag project.thumbnail
      end
      project.screenshots.each do |screenshot|
        row :screenshot do
          image_tag screenshot
        end
      end
    end
  end

  form do |f|
    f.semantic_errors # shows errors on :base
    f.inputs          # builds an input field for every attribute
    f.actions         # adds the 'Submit' and 'Cancel' buttons
    panel 'Images' do
      render partial: 'images', locals: { project: }
    end
  end
end
