# frozen_string_literal: true

ActiveAdmin.register User do
  actions :all, except: [:destroy]

  permit_params :username, :is_root, :is_developer, :is_node_operator, :password, :password_confirmation
  includes :authorizations

  index do
    selectable_column
    id_column
    column :username
    column :is_developer
    column :is_node_operator
    column :providers
    column 'Last Sign In', :current_sign_in_at
    column :sign_in_count
    column :created_at
    actions
  end

  filter :username
  filter :sign_in_count
  filter :is_root
  filter :is_developer
  filter :is_node_operator
  filter :created_at

  show do
    default_main_content do
      row :providers
    end
  end

  form do |f|
    f.inputs do
      f.input :username
      f.input :is_root
      f.input :password
      f.input :password_confirmation
    end
    f.actions
  end
end
