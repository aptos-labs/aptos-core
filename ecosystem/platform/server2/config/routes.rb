# frozen_string_literal: true

Rails.application.routes.draw do
  devise_for :users, ActiveAdmin::Devise.config.deep_merge(controllers: {
                                                             omniauth_callbacks: 'api/users/omniauth_callbacks'
                                                           })
  ActiveAdmin.routes(self)
  # For details on the DSL available within this file, see https://guides.rubyonrails.org/routing.html

  get 'auth/:provider/callback', to: 'sessions#create'
  get '/login', to: 'sessions#new'

  namespace :api do
    get 'users/me', to: 'users#me'
    resources :users, only: %i[show update]
  end
end
