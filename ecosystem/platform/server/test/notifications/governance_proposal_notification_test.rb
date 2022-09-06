# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

require 'test_helper'

class GovernanceProposalNotificationTest < ActiveSupport::TestCase
  include ActionMailer::TestHelper

  test 'is not delivered if preferences don\'t exist' do
    user = FactoryBot.create(:user)
    network_operation = NetworkOperation.create(title: 'foo', content: 'bar')
    notification = GovernanceProposalNotification.with(network_operation:)

    assert_no_difference('Notification.count') do
      assert_emails 0 do
        notification.deliver(user)
      end
    end
  end

  test 'is delivered if preference is true' do
    user = FactoryBot.create(:user)
    NotificationPreference.create(user:, delivery_method: :database, governance_proposal_notification: true)
    NotificationPreference.create(user:, delivery_method: :email, governance_proposal_notification: true)
    network_operation = NetworkOperation.create(title: 'foo', content: 'bar')
    notification = GovernanceProposalNotification.with(network_operation:)

    assert_difference('Notification.count') do
      assert_emails 1 do
        notification.deliver(user)
      end
    end
  end

  test 'is not delivered by email if preference is true but user does not have a confirmed email' do
    user = FactoryBot.create(:user, email: nil, unconfirmed_email: Faker::Internet.email)
    NotificationPreference.create(user:, delivery_method: :database, governance_proposal_notification: true)
    NotificationPreference.create(user:, delivery_method: :email, governance_proposal_notification: true)
    network_operation = NetworkOperation.create(title: 'foo', content: 'bar')
    notification = GovernanceProposalNotification.with(network_operation:)

    assert_difference('Notification.count') do
      assert_emails 0 do
        notification.deliver(user)
      end
    end
  end

  test 'is not delivered if preference is false' do
    user = FactoryBot.create(:user)
    NotificationPreference.create(user:, delivery_method: :database, governance_proposal_notification: false)
    NotificationPreference.create(user:, delivery_method: :email, governance_proposal_notification: false)
    network_operation = NetworkOperation.create(title: 'foo', content: 'bar')
    notification = GovernanceProposalNotification.with(network_operation:)

    assert_no_difference('Notification.count') do
      assert_emails 0 do
        notification.deliver(user)
      end
    end
  end
end
