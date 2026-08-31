#!/usr/bin/env ruby
# Set up TestFlight for the app: the tester-facing blurb, the details Beta App Review asks for, and
# the two groups. None of this needs a build to exist, so it can all be in place before the first
# upload and the build then only has to be assigned to a group.
#
#   APPLE_EMAIL=you@example.com ruby scripts/testflight-setup.rb
begin
  require "spaceship"
rescue LoadError
  libexec = Dir["/opt/homebrew/Cellar/fastlane/*/libexec", "/usr/local/Cellar/fastlane/*/libexec"].max
  abort "spaceship is not installed. `brew install fastlane` and rerun." unless libexec
  ENV["GEM_PATH"] = [libexec, ENV["GEM_PATH"]].compact.join(":")
  Gem.clear_paths
  require "spaceship"
end

DIR = ENV.fetch("METADATA_DIR", "appstore/metadata")
LOCALE = "en-US"
BUNDLE_ID = ENV.fetch("BUNDLE_ID", "studio.margin.app")
ENV["FASTLANE_ITC_TEAM_ID"] = ENV.fetch("FASTLANE_ITC_TEAM_ID", "129377371")

def field(name, localized: true)
  path = localized ? File.join(DIR, LOCALE, "#{name}.txt") : File.join(DIR, "#{name}.txt")
  File.exist?(path) ? File.read(path).strip : nil
end

Spaceship::ConnectAPI.login(ENV["APPLE_EMAIL"], nil, use_portal: false, use_tunes: true)

app = Spaceship::ConnectAPI::App.find(BUNDLE_ID)
abort "No app for #{BUNDLE_ID}." unless app
puts "#{app.name} (#{app.id})"

localization = app.get_beta_app_localizations.find { |l| l.locale == LOCALE }
attributes = {
  description: field("beta_description"),
  feedbackEmail: field("beta_feedback_email", localized: false),
  marketingUrl: field("marketing_url"),
  privacyPolicyUrl: field("privacy_url"),
}

if localization
  Spaceship::ConnectAPI.patch_beta_app_localizations(localization_id: localization.id, attributes: attributes)
else
  Spaceship::ConnectAPI.post_beta_app_localizations(app_id: app.id, attributes: attributes.merge(locale: LOCALE))
end
puts "  tester blurb and feedback address set"

# Apple requires a contact phone number here, and this only gates external testing: internal
# testers never go through Beta App Review. So a missing number is a warning, not a failure.
if field("review_phone", localized: false)
  Spaceship::ConnectAPI.patch_beta_app_review_detail(app_id: app.id, attributes: {
    contactFirstName: field("review_first_name", localized: false),
    contactLastName: field("review_last_name", localized: false),
    contactEmail: field("review_email", localized: false),
    contactPhone: field("review_phone", localized: false),
    # Margin has no accounts at all, so there is nothing for a reviewer to sign in to. Saying so
    # explicitly is what stops the review coming back asking for credentials.
    demoAccountRequired: false,
    notes: field("review_notes", localized: false),
  })
  puts "  beta app review contact and notes set"
else
  puts "  skipping beta app review details: #{DIR}/review_phone.txt is missing and Apple requires"
  puts "  a contact number. Internal testing works without it; external testing does not."
end

existing = app.get_beta_groups.map(&:name)

[
  { name: "Internal", internal: true, public_link: false },
  { name: "Public Beta", internal: false, public_link: true },
].each do |group|
  if existing.include?(group[:name])
    puts "  group #{group[:name].inspect} already exists"
    next
  end

  if group[:internal]
    # spaceship always sends the public-link attributes, and App Store Connect rejects them
    # outright on an internal group rather than ignoring them, so this one is posted by hand.
    body = {
      data: {
        attributes: { name: group[:name], isInternalGroup: true, hasAccessToAllBuilds: true },
        relationships: { app: { data: { id: app.id, type: "apps" } } },
        type: "betaGroups",
      },
    }
    Spaceship::ConnectAPI.client.test_flight_request_client.post("v1/betaGroups", body)
    puts "  created internal group #{group[:name].inspect}"
  else
    created = app.create_beta_group(
      group_name: group[:name],
      is_internal_group: false,
      public_link_enabled: true,
      public_link_limit_enabled: true,
    )
    puts "  created external group #{created.name.inspect}"
  end
end

puts
app.get_beta_groups.each do |g|
  kind = g.is_internal_group ? "internal" : "external"
  puts "  #{g.name} (#{kind})#{g.public_link ? " #{g.public_link}" : ''}"
end
