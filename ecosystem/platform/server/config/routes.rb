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

  get 'onboarding/email', to: 'onboarding#email'
  post 'onboarding/email', to: 'onboarding#email_update'

  resources :overview, only: %i[index]
  resources :it1_profiles, except: %i[show index destroy]

  get 'it1', to: 'welcome#it1'
  root 'welcome#index'
end
