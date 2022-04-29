# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

Rails.application.routes.draw do
  devise_for :users, **ActiveAdmin::Devise.config.deep_merge(controllers: {
                                                               omniauth_callbacks: 'users/omniauth_callbacks'
                                                             }), path: :users
  ActiveAdmin.routes(self)
  # Define your application routes per the DSL in https://guides.rubyonrails.org/routing.html

  namespace :api do
    # get ':provider/callback', to: 'sessions#create'

    get 'users/me', to: 'users#me'
    resources :users, only: %i[show update]
  end

  # Defines the root path route ("/")
  # TODO: make this the static rails renderer
  # root "articles#index"
end
