# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

class UserMailerPreview < ActionMailer::Preview
  def node_upgrade_notification
    recipient = FactoryBot.build(:user)
    network_operation = NetworkOperation.new(
      title: 'Upgrade your Node!',
      content: '
      <p>The node needs to be upgraded for the following reasons:</p>
      <ul>
      <li>Lorem</li>
      <li>Ipsum</li>
      <li>Dolor</li>
      <li>Sit</li>
      <li>Amet</li>
      </ul>
      '
    )
    UserMailer.with(recipient:, network_operation:).node_upgrade_notification
  end

  def governance_proposal_notification
    recipient = FactoryBot.build(:user)
    network_operation = NetworkOperation.new(
      title: 'Governance Proposal',
      content: '
      <p>We the people, in order to form a more perfect union,</p>
      <ul>
      <li>establish justice</li>
      <li>insure domestic tranquility</li>
      <li>provide for the common defense</li>
      <li>promote the general welfare</li>
      <li>and secure the blessings of liberty to ourselves and our posterity</li>
      </ul>
      <p>do ordain and establish this Governance Proposal.</p>
      '
    )
    UserMailer.with(recipient:, network_operation:).governance_proposal_notification
  end
end
