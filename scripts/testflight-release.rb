#!/usr/bin/env ruby
# Put the most recently uploaded build in front of testers.
#
# Apple takes somewhere between a few minutes and an hour to process an upload, and nothing can be
# assigned until it has. So this waits rather than failing, and says what it is waiting for.
#
#   APPLE_EMAIL=you@example.com ruby scripts/testflight-release.rb
#
# Internal testers get the build as soon as it is assigned. External testers are gated on Beta App
# Review, which this submits for and which usually comes back within a day.
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
WAIT_SECONDS = Integer(ENV.fetch("WAIT_SECONDS", "1800"))

Spaceship::ConnectAPI.login(ENV["APPLE_EMAIL"], nil, use_portal: false, use_tunes: true)

app = Spaceship::ConnectAPI::App.find(BUNDLE_ID)
abort "No app for #{BUNDLE_ID}." unless app
puts "#{app.name} (#{app.id})"

deadline = Time.now + WAIT_SECONDS
build = nil

loop do
  builds = Spaceship::ConnectAPI::Build.all(app_id: app.id, sort: "-uploadedDate", limit: 5)
  ready = builds.reject(&:expired).find { |b| b.processing_state == "VALID" }

  if ready
    build = ready
    break
  end

  pending = builds.find { |b| b.processing_state == "PROCESSING" }
  if Time.now > deadline
    abort "Timed out after #{WAIT_SECONDS}s. #{pending ? 'The build is still processing.' : 'No build has appeared yet.'}"
  end

  puts(pending ? "  waiting: build #{pending.version} is still processing" : "  waiting: no build has appeared yet")
  sleep(30)
end

puts "  build #{build.app_version} (#{build.version}) is ready"

groups = app.get_beta_groups

# A group created with hasAccessToAllBuilds receives every build the moment it processes, and Apple
# rejects an explicit assignment to one rather than treating it as a no-op. The internal group is
# exactly that, so there is nothing to do for it and nothing to report either.
assignable = groups.reject { |g| g.is_internal_group || g.has_access_to_all_builds }
already = (build.get_beta_groups.map(&:id) rescue [])
to_add = assignable.reject { |g| already.include?(g.id) }

if to_add.empty?
  puts "  nothing to assign: #{groups.map(&:name).join(', ')} already have this build"
else
  build.add_beta_groups(beta_groups: to_add)
  puts "  assigned to #{to_add.map(&:name).join(', ')}"
end

# Only external groups are gated on review, so an app with internal testers only never needs this.
if groups.any? { |g| !g.is_internal_group }
  begin
    Spaceship::ConnectAPI.post_beta_app_review_submissions(build_id: build.id)
    puts "  submitted for beta app review, which gates the external testers"
  rescue => e
    # Resubmitting an already submitted build is not an error worth failing the run over.
    puts "  beta app review not submitted: #{e.message.lines.first.to_s.strip}"
  end
end

# The store version needs the build attached too, and that is a separate thing from TestFlight.
# Until it is, App Store Connect shows the listing with no app icon, because for a Mac app the icon
# is read out of the attached build rather than uploaded alongside the other artwork.
version = app.get_edit_app_store_version(platform: Spaceship::ConnectAPI::Platform::MAC_OS)
if version.nil?
  puts "  no editable store version to attach the build to"
elsif version.build&.id == build.id
  puts "  already attached to store version #{version.version_string}"
else
  version.select_build(build_id: build.id)
  puts "  attached to store version #{version.version_string}, which is what surfaces the app icon"
end

puts
puts "Internal testers can install now. External testers wait on beta app review."
