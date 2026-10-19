#!/usr/bin/env ruby
# Create the Apple Developer resources a release needs, and turn them into files CI can use.
#
# Everything here is find-or-create, so running it twice does nothing the second time. That matters
# most for the Developer ID certificate: an account may hold only a handful, they cannot be
# un-revoked, and every copy of the app already signed by one stops verifying if it goes away.
#
# The private keys are never sent to Apple and never leave ~/.margin-signing. Apple only ever sees
# the certificate signing requests, which is the whole point of generating them with openssl up
# front rather than letting a tool make its own.
#
#   BUNDLE_ID=studio.margin.app APP_NAME=Margin ruby scripts/apple-provision.rb
#
# Run it yourself rather than through an agent: the Apple ID password and the two-factor code are
# prompted for on the terminal.
begin
  require "spaceship"
rescue LoadError
  # Homebrew vendors fastlane's gems under libexec instead of putting them on the default gem
  # path, so spaceship is not requirable until that directory is added to it.
  libexec = Dir["/opt/homebrew/Cellar/fastlane/*/libexec", "/usr/local/Cellar/fastlane/*/libexec"].max
  abort "spaceship is not installed. `brew install fastlane` and rerun." unless libexec
  ENV["GEM_PATH"] = [libexec, ENV["GEM_PATH"]].compact.join(":")
  Gem.clear_paths
  require "spaceship"
end

require "openssl"
require "fileutils"
require "securerandom"
require "net/http"
require "tmpdir"

# Listing profiles otherwise goes through developerservices2.apple.com, Apple's Xcode-only
# endpoint, which rejects a plain spaceship session with "Please update to Xcode 7.3 or later"
# no matter how current Xcode actually is. This routes it back to the ordinary portal API.
ENV["SPACESHIP_AVOID_XCODE_API"] = "1"

DIR = File.expand_path("~/.margin-signing")
BUNDLE_ID = ENV.fetch("BUNDLE_ID", "studio.margin.app")
APP_NAME  = ENV.fetch("APP_NAME", "Margin")
EMAIL     = ENV["APPLE_EMAIL"]

# Apple's intermediates. codesign builds a chain from the leaf up, so a .p12 holding only the leaf
# and its key fails on a fresh CI keychain with "unable to build chain to self-signed root".
INTERMEDIATES = {
  "AppleWWDRCAG3" => "https://www.apple.com/certificateauthority/AppleWWDRCAG3.cer",
  "DeveloperIDG2CA" => "https://www.apple.com/certificateauthority/DeveloperIDG2CA.cer",
}

CERTS = [
  { key: "developer-id",       klass: Spaceship::Portal::Certificate::DeveloperIdApplication,
    label: "Developer ID Application (direct download, notarized)",       ca: "DeveloperIDG2CA" },
  { key: "apple-distribution", klass: Spaceship::Portal::Certificate::MacAppDistribution,
    label: "Mac App Distribution (App Store .app)",                       ca: "AppleWWDRCAG3" },
  { key: "mac-installer",      klass: Spaceship::Portal::Certificate::MacInstallerDistribution,
    label: "Mac Installer Distribution (App Store .pkg)",                 ca: "AppleWWDRCAG3" },
]

def common_name(cert)
  cert.subject.to_a.find { |n, _, _| n == "CN" }&.at(1)
end

def fetch_intermediate(name)
  path = File.join(DIR, "#{name}.cer")
  unless File.exist?(path)
    uri = URI(INTERMEDIATES.fetch(name))
    File.binwrite(path, Net::HTTP.get(uri))
  end
  OpenSSL::X509::Certificate.new(File.binread(path))
end

# Confirm macOS can actually read the bundle, because the failure mode otherwise shows up days
# later inside a CI keychain as "wrong password" rather than as anything about the format.
def importable?(p12_path, password)
  keychain = File.join(Dir.tmpdir, "margin-p12-check-#{SecureRandom.hex(4)}.keychain-db")
  system("security", "create-keychain", "-p", "check", keychain, out: File::NULL, err: File::NULL)
  system("security", "unlock-keychain", "-p", "check", keychain, out: File::NULL, err: File::NULL)
  ok = system("security", "import", p12_path, "-k", keychain, "-P", password,
              out: File::NULL, err: File::NULL)
  system("security", "delete-keychain", keychain, out: File::NULL, err: File::NULL)
  ok
end

# Returns nil on success, or a sentence saying what went wrong.
def write_p12(spec, cert)
  key_path = File.join(DIR, "#{spec[:key]}.key")
  unless cert.check_private_key(OpenSSL::PKey::RSA.new(File.read(key_path)))
    return "the issued certificate does not match the local private key, so Apple issued it " \
           "against a different CSR. Revoke it in the portal and rerun."
  end

  password = SecureRandom.hex(24)
  p12_path = File.join(DIR, "#{spec[:key]}.p12")

  built = Dir.mktmpdir do |tmp|
    leaf = File.join(tmp, "leaf.pem")
    ca = File.join(tmp, "ca.pem")
    File.write(leaf, cert.to_pem)
    File.write(ca, fetch_intermediate(spec[:ca]).to_pem)

    # Ruby links OpenSSL 3, whose PKCS12 default MAC is SHA-256. Apple's Security framework reads
    # only the legacy SHA-1 MAC and reports the mismatch as a wrong password, so the bundle has to
    # come from the LibreSSL at /usr/bin/openssl, which still writes the older format. The password
    # goes through the environment rather than argv so it stays out of the process list.
    system({ "P12PASS" => password }, "/usr/bin/openssl", "pkcs12", "-export",
           "-inkey", key_path, "-in", leaf, "-certfile", ca,
           "-name", common_name(cert), "-passout", "env:P12PASS", "-out", p12_path,
           out: File::NULL, err: File::NULL)
  end

  return "/usr/bin/openssl could not build the bundle." unless built
  return "macOS refused to import the bundle that was just built." unless importable?(p12_path, password)

  File.write(File.join(DIR, "#{spec[:key]}.p12.pass"), password)
  File.chmod(0o600, p12_path, File.join(DIR, "#{spec[:key]}.p12.pass"))
  nil
end

FileUtils.mkdir_p(DIR)
Spaceship::Portal.login(EMAIL)
Spaceship::Portal.select_team
team_id = Spaceship::Portal.client.team_id
puts "Team ID: #{team_id}"
puts

identities = {}

CERTS.each do |spec|
  existing = spec[:klass].all.select { |c| c.status == "Issued" }
  cert_obj = existing.first

  if cert_obj
    puts "#{spec[:label]}: already exists (#{cert_obj.id}), not creating another."
  else
    csr = File.read(File.join(DIR, "#{spec[:key]}.csr"))
    cert_obj = spec[:klass].create!(csr: csr)
    puts "#{spec[:label]}: created (#{cert_obj.id})."
  end

  x509 = cert_obj.download
  File.binwrite(File.join(DIR, "#{spec[:key]}.cer"), x509.to_der)

  identities[spec[:key]] = common_name(x509)
  puts "  identity: #{common_name(x509)}"

  problem = write_p12(spec, x509)
  puts(problem ? "  no p12: #{problem}" : "  wrote #{spec[:key]}.p12")
  puts
end

app = Spaceship::Portal::App.find(BUNDLE_ID, mac: true)
if app
  puts "App ID #{BUNDLE_ID}: already registered."
else
  app = Spaceship::Portal::App.create!(bundle_id: BUNDLE_ID, name: APP_NAME, mac: true)
  puts "App ID #{BUNDLE_ID}: registered."
end

profile_name = "#{APP_NAME} App Store"
profile = Spaceship::Portal::ProvisioningProfile::AppStore.all(mac: true).find do |p|
  p.app.bundle_id == BUNDLE_ID && p.status == "Active"
end

if profile
  puts "Provisioning profile: reusing #{profile.name}."
else
  mas_cert = Spaceship::Portal::Certificate::MacAppDistribution.all.first
  profile = Spaceship::Portal::ProvisioningProfile::AppStore.create!(
    name: profile_name, bundle_id: BUNDLE_ID, certificate: mas_cert, mac: true
  )
  puts "Provisioning profile: created #{profile.name}."
end

profile_path = File.join(DIR, "#{BUNDLE_ID}.provisionprofile")
File.binwrite(profile_path, profile.download)
File.chmod(0o600, profile_path)

File.write(File.join(DIR, "#{BUNDLE_ID}.env"), <<~ENV)
  APPLE_TEAM_ID="#{team_id}"
  APPLE_SIGNING_IDENTITY="#{identities['developer-id']}"
  MAS_APP_IDENTITY="#{identities['apple-distribution']}"
  MAS_INSTALLER_IDENTITY="#{identities['mac-installer']}"
ENV

puts
puts "Wrote #{profile_path} and #{BUNDLE_ID}.env into #{DIR}."
puts "Still to do by hand, because Apple has no API for it: create an App Store Connect API key"
puts "(Users and Access, Integrations) and save the .p8 as #{DIR}/AuthKey.p8."
