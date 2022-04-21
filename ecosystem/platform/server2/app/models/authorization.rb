# frozen_string_literal: true

class Authorization < ApplicationRecord
  belongs_to :user, optional: true

  validates_uniqueness_of :uid, scope: [:provider]
end
