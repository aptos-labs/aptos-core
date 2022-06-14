# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

require 'httparty'

class PersonaURL
  # @param [URI] parsed_url
  def initialize(parsed_url)
    @parsed = parsed_url
    query = parsed_url.query
    @query = if query.present?
               CGI.parse(query)
             else
               {}
             end
  end

  # @param [String] key
  # @param [Object] value
  # @return [PersonaURL]
  def set_param(key, value)
    @query[key] = value
    self
  end

  def to_s
    @parsed.query = URI.encode_www_form(@query)
    @parsed.to_s
  end
end

module PersonaHelper
  FirstInquiryResult = Struct.new(:exists?, :inquiry_id, :status)

  class PersonaClient
    include HTTParty
    # debug_output $stdout

    base_uri 'https://withpersona.com/api/v1/'
    headers({ 'Authorization' => "Bearer #{ENV.fetch('PERSONA_API_KEY', 'test')}", 'Persona-Version' => '2021-05-14',
              'Key-Inflection' => 'snake_case' })

    # TODO: support idempotency key (user.external_id is fine)

    # @param [String] reference_id
    # @return [FirstInquiryResult, nil]
    def inquiry_id_from_reference_id(reference_id)
      inquiries = get_inquiries(reference_id:)
      first_inquiry = inquiries['data']&.first
      if first_inquiry.present?
        FirstInquiryResult.new(true, first_inquiry['id'], first_inquiry['attributes']['status'])
      else
        FirstInquiryResult.new(false, nil, nil)
      end
    end

    # Declined = watchlist failed, Approved = watchlist passed!. Nil = neither! (no watchlist run)
    # @param [User] user
    # @return [TrueClass, FalseClass, nil]
    def user_watchlist_checks_passed(user)
      inquiries = get_inquiries(reference_id: user.external_id)
      inquiries['data']&.each do |inquiry|
        case inquiry['attributes']['status']
        when 'declined'
          return false
        when 'approved'
          return true
        end
      end
      nil
    end

    # https://docs.withpersona.com/reference/list-all-inquiries
    # @param [Integer] after
    # @param [String] account_id
    # @param [String] reference_id
    def get_inquiries(after: nil, account_id: nil, reference_id: nil)
      query = { 'page[size]' => 50 }
      query['page[after]'] = after if after.present?
      query['filter[account-id]'] = account_id if account_id.present?
      query['filter[reference-id]'] = reference_id if reference_id.present?
      self.class.get('/inquiries', { query: })
    end

    # https://docs.withpersona.com/reference/apiv1inquiriesinquiry-id
    def inquiry(inquiry_id)
      self.class.get("/inquiries/#{inquiry_id}")
    end
  end

  class PersonaInvite
    # @param [User] user
    def initialize(user)
      @user = user
    end

    def url
      PersonaURL.new(URI.parse(base_url))
                .set_param('reference-id', @user.external_id)
    end

    private

    def base_url
      ENV.fetch('PERSONA_URL_PREFIX', ' https://withpersona.com/verify?inquiry-template-id=itmpl_X1pKpnefCS8wShXJBtXyfRSf&environment=sandbox')
    end
  end
end
