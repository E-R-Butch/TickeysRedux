cask "tickeys-redux" do
  version "1.0.0"
  sha256 "aba2f1b3152b30a3b81091403c92096b946905ed4db73ffa9f5d856bd142c90c"

  url "https://github.com/E-R-Butch/TickeysRedux/releases/download/v#{version}/Tickeys.Redux.v#{version}.dmg"
  name "Tickeys Redux"
  desc "Instant audio feedback for every keystroke — mechanical keyboard sounds"
  homepage "https://github.com/E-R-Butch/TickeysRedux"

  depends_on arch: :arm64

  app "Tickeys Redux.app"

  zap trash: [
    "~/Library/Preferences/com.tickeys.redux.plist",
    "~/Library/Saved Application State/com.tickeys.redux.savedState",
  ]

  caveats <<~EOS
    Tickeys Redux is Apple Silicon only. Intel Mac users should use the
    original Tickeys: brew install --cask tickeys

    After launching, grant Input Monitoring permission in
    System Settings → Privacy & Security → Input Monitoring.
  EOS
end
