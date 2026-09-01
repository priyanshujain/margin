#!/usr/bin/env ruby
# Push the App Review Information for the store submission: who to contact, and the notes that
# explain anything a reviewer would otherwise have to guess at.
#
#   APPLE_EMAIL=you@example.com ruby scripts/appstore-review-detail.rb
#
# This is a different record from the TestFlight one that testflight-setup.rb writes. Beta review
# and store review do not share notes, so an explanation that only went to TestFlight is invisible
# to the reviewer looking at the store submission, and to the automated entitlement check that runs
# before a human sees it at all.
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
BUNDLE_ID = ENV.fetch("BUNDLE_ID", "studio.margin.app")
ENV["FASTLANE_ITC_TEAM_ID"] = ENV.fetch("FASTLANE_ITC_TEAM_ID", "129377371")

NOTES_LIMIT = 4000

def field(name)
  path = File.join(DIR, "#{name}.txt")
  return nil unless File.exist?(path)

  File.read(path).strip
end

notes = field("review_notes")
abort "#{DIR}/review_notes.txt is missing." unless notes
abort "review_notes is #{notes.length} characters, over Apple's limit of #{NOTES_LIMIT}." if notes.length > NOTES_LIMIT

phone = field("review_phone")
abort "#{DIR}/review_phone.txt is missing and Apple requires a contact number." unless phone

Spaceship::ConnectAPI.login(ENV["APPLE_EMAIL"], nil, use_portal: false, use_tunes: true)

app = Spaceship::ConnectAPI::App.find(BUNDLE_ID)
abort "No app for #{BUNDLE_ID}." unless app
puts "#{app.name} (#{app.id})"

version = app.get_edit_app_store_version(platform: Spaceship::ConnectAPI::Platform::MAC_OS)
abort "No editable macOS version; the submission may already be in review." unless version
puts "  version #{version.version_string} (#{version.app_store_state})"

attributes = {
  contactFirstName: field("review_first_name"),
  contactLastName: field("review_last_name"),
  contactEmail: field("review_email"),
  contactPhone: phone,
  # Margin has no accounts at all, so there is nothing for a reviewer to sign in to. Saying so
  # explicitly is what stops the review coming back asking for credentials.
  demoAccountRequired: false,
  notes: notes,
}

detail = version.fetch_app_store_review_detail

if detail
  Spaceship::ConnectAPI.patch_app_store_review_detail(
    app_store_review_detail_id: detail.id,
    attributes: attributes,
  )
else
  Spaceship::ConnectAPI.post_app_store_review_detail(
    app_store_version_id: version.id,
    attributes: attributes,
  )
end

# Apple accepts a patch it then stores as something else often enough to be worth reading back, and
# an empty notes field is exactly the state that got this submission rejected in the first place.
written = version.fetch_app_store_review_detail&.notes.to_s
if written.empty?
  abort "  the notes field came back empty; nothing was saved."
elsif written != notes
  puts "  saved, but what came back differs from what was sent. Check it in App Store Connect."
else
  puts "  contact and #{notes.length} characters of review notes saved"
end
