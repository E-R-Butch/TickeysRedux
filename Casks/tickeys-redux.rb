cask "tickeys-redux" do
  version "1.0.7"
  sha256 "85831f700b85575c8d72fb09c20ccc363b1a560dc5e85bbde8a67aec87f82fba"

  url "https://github.com/E-R-Butch/TickeysRedux/releases/download/v#{version}/Tickeys.Redux.v#{version}.dmg"
  name "Tickeys Redux"
  desc "Instant audio feedback for every keystroke — mechanical keyboard sounds"
  homepage "https://github.com/E-R-Butch/TickeysRedux"

  depends_on arch: :arm64
  depends_on macos: :ventura

  app "Tickeys Redux.app"

  zap trash: [
    "~/Library/Preferences/com.sinclair.tickeys-redux.plist",
    "~/Library/Saved Application State/com.sinclair.tickeys-redux.savedState",
  ]

  caveats <<~EOS
    Tickeys Redux is Apple Silicon only. Intel Mac users should use the
    original Tickeys: brew install --cask tickeys

    This build uses the project's free self-signed identity, not Apple
    Developer ID notarization. If macOS blocks the first launch, Control-click
    Tickeys Redux.app and choose Open.

    When upgrading from an older ad-hoc build, remove the stale Tickeys Redux
    entry from Input Monitoring, add the current app, and enable it once.

    After launching, grant Input Monitoring permission in
    System Settings → Privacy & Security → Input Monitoring.
  EOS
end
