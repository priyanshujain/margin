#!/usr/bin/env ruby
# Declare the age rating and set pricing. These are the two things App Store Connect will not let a
# submission through without, and neither has anything to do with the copy in appstore-listing.rb.
#
#   APPLE_EMAIL=you@example.com ruby scripts/appstore-compliance.rb
#
# Every content answer here is NONE because Margin is an editor for words the person using it wrote
# themselves. It ships no media, has no feed, no other users, and no in-app browser. If any of that
# ever stops being true, this file is the thing that has to change with it.
begin
  require "spaceship"
rescue LoadError
  libexec = Dir["/opt/homebrew/Cellar/fastlane/*/libexec", "/usr/local/Cellar/fastlane/*/libexec"].max
  abort "spaceship is not installed. `brew install fastlane` and rerun." unless libexec
  ENV["GEM_PATH"] = [libexec, ENV["GEM_PATH"]].compact.join(":")
  Gem.clear_paths
  require "spaceship"
end

BUNDLE_ID = ENV.fetch("BUNDLE_ID", "studio.margin.app")
ENV["FASTLANE_ITC_TEAM_ID"] = ENV.fetch("FASTLANE_ITC_TEAM_ID", "129377371")

NONE = Spaceship::ConnectAPI::AgeRatingDeclaration::Rating::NONE

Spaceship::ConnectAPI.login(ENV["APPLE_EMAIL"], nil, use_portal: false, use_tunes: true)

app = Spaceship::ConnectAPI::App.find(BUNDLE_ID)
abort "No app for #{BUNDLE_ID}." unless app
puts "#{app.name} (#{app.id})"

info = app.fetch_edit_app_info
abort "No editable app info." unless info

declaration = info.fetch_age_rating_declaration
abort "No age rating declaration to write to." unless declaration

graded = {
  alcoholTobaccoOrDrugUseOrReferences: NONE,
  contests: NONE,
  gamblingSimulated: NONE,
  gunsOrOtherWeapons: NONE,
  horrorOrFearThemes: NONE,
  matureOrSuggestiveThemes: NONE,
  medicalOrTreatmentInformation: NONE,
  profanityOrCrudeHumor: NONE,
  sexualContentGraphicAndNudity: NONE,
  sexualContentOrNudity: NONE,
  violenceCartoonOrFantasy: NONE,
  violenceRealistic: NONE,
  violenceRealisticProlongedGraphicOrSadistic: NONE,
}

# The app opens links in the system browser rather than rendering the web itself, and the only
# content it ever shows is the person's own writing, so there is no unrestricted web access and no
# user generated content in the sense Apple means: content from other people.
boolean = {
  advertising: false,
  ageAssurance: false,
  gambling: false,
  healthOrWellnessTopics: false,
  lootBox: false,
  messagingAndChat: false,
  parentalControls: false,
  unrestrictedWebAccess: false,
  userGeneratedContent: false,
}

declaration.update(attributes: graded.merge(boolean))
puts "  age rating declared, every content question answered NONE"

# App Privacy. Margin has no telemetry, no accounts and no server of its own, so nothing is
# collected. The Google Drive backup is the one thing that sends bytes anywhere, and it sends them
# to the account of the person who turned it on, which is not the developer collecting anything.
usages = Spaceship::ConnectAPI::AppDataUsage.all(
  app_id: app.id, includes: "category,grouping,purpose,dataProtection"
)

if usages.any?(&:is_not_collected?)
  puts "  privacy already declared as data not collected"
else
  Spaceship::ConnectAPI::AppDataUsage.create(app_id: app.id, app_data_usage_protection_id: "DATA_NOT_COLLECTED")
  puts "  privacy declared: data not collected"
end

state = Spaceship::ConnectAPI::AppDataUsagesPublishState.get(app_id: app.id)
if state.published
  puts "  privacy declaration already published"
else
  state.publish!
  puts "  privacy declaration published"
end

# Free, everywhere. Territory availability is left alone: the default is all of them.
app.update(attributes: { pricing: [] }) if ENV["SET_PRICING"]
puts "  pricing left as configured (the app is free, which is the default for a new record)"
