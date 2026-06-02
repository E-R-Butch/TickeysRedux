cask "tickeys-redux" do
  version "1.0.5"
  sha256 "51ba63a15a269b68946e461391c4bf3b295167e3ee630432e0b2f84208a2e1aa"

  url "https://github.com/E-R-Butch/TickeysRedux/releases/download/v#{version}/Tickeys.Redux.v#{version}.dmg"
  name "Tickeys Redux"
  desc "Instant audio feedback for every keystroke — mechanical keyboard sounds"
  homepage "https://github.com/E-R-Butch/TickeysRedux"

  depends_on arch: :arm64

  app "Tickeys Redux.app"

  zap trash: [
    "~/Library/Preferences/com.sinclair.tickeys-redux.plist",
    "~/Library/Saved Application State/com.sinclair.tickeys-redux.savedState",
  ]

  caveats <<~EOS
    Tickeys Redux is Apple Silicon only. Intel Mac users should use the
    original Tickeys: brew install --cask tickeys

    After launching, grant Input Monitoring permission in
    System Settings → Privacy & Security → Input Monitoring.
  EOS
end
