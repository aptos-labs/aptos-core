# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

Rails.application.routes.draw do
  devise_for :users, {
    controllers: {
      omniauth_callbacks: 'users/omniauth_callbacks'
    }
  }
  ActiveAdmin.routes(self)

  # Define your application routes per the DSL in https://guides.rubyonrails.org/routing.html

  namespace :api do
    # get ':provider/callback', to: 'sessions#create'
    get 'users/me', to: 'users#me'
    resources :users, only: %i[show update]
  end

  namespace :user do
    root to: redirect('/it1') # creates user_root_path, where users go after confirming email
  end

  # KYC routes
  get 'onboarding/kyc_redirect', to: 'onboarding#kyc_redirect'
  get 'onboarding/kyc_callback', to: 'onboarding#kyc_callback'

  get 'onboarding/email', to: 'onboarding#email'
  post 'onboarding/email', to: 'onboarding#email_update'

  get 'health', to: 'health#health'

  resources :overview, only: %i[index]
  resources :it1_profiles, except: %i[index destroy]

  get 'it1', to: 'welcome#it1'
  root 'welcome#index'
end
