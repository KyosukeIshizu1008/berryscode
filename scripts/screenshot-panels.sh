#!/usr/bin/env bash
# Capture every left-panel state of BerryCode for visual review.
#
# macOS only. Uses the Ctrl+1..7 panel-switch shortcuts defined in
# berrycode/src/app/shortcuts.rs plus the built-in `screencapture` to
# save a PNG per panel. No external deps.
#
# Usage:
#   ./scripts/screenshot-panels.sh
#   OUT_DIR=/tmp/foo ./scripts/screenshot-panels.sh
#
# Requires: BerryCode already running. Terminal needs Accessibility
# permission for "System Events" key events to reach the editor
# (System Settings → Privacy & Security → Accessibility).

set -euo pipefail

OUT_DIR="${OUT_DIR:-docs/screenshots/$(date +%Y%m%d-%H%M%S)}"
mkdir -p "$OUT_DIR"

PROC="${BERRYCODE_PROC:-berrycode}"
if ! pgrep -x "$PROC" >/dev/null 2>&1; then
  echo "Error: '$PROC' is not running. Start it first:" >&2
  echo "  cargo run --bin berrycode" >&2
  exit 1
fi

# Bring the window to front and read its bounds (screen coords).
read -r WIN_X WIN_Y WIN_W WIN_H < <(osascript <<'OSA'
tell application "System Events"
  tell process "berrycode"
    set frontmost to true
    delay 0.3
    set p to position of window 1
    set s to size of window 1
    return (item 1 of p as string) & " " & (item 2 of p as string) & " " & ¬
           (item 1 of s as string) & " " & (item 2 of s as string)
  end tell
end tell
OSA
)

echo "Window: ${WIN_W}x${WIN_H} at (${WIN_X},${WIN_Y})"
echo "Output: $OUT_DIR"
echo

# Panel list — order matches MAIN_PANELS in app/mod.rs and the
# Ctrl+<digit> shortcuts in app/shortcuts.rs.
PANELS=(
  "1:explorer"
  "2:search"
  "3:git"
  "4:terminal"
  "5:ecs_inspector"
  "6:bevy_templates"
  "7:scene_editor"
)

for entry in "${PANELS[@]}"; do
  key="${entry%%:*}"
  name="${entry##*:}"

  # Send Ctrl+<digit> to the editor.
  osascript -e "tell application \"System Events\" to tell process \"$PROC\" to keystroke \"$key\" using control down" >/dev/null
  sleep 0.4  # let egui repaint

  screencapture -x -R "${WIN_X},${WIN_Y},${WIN_W},${WIN_H}" "$OUT_DIR/${name}.png"
  printf "  %-18s → %s\n" "$name" "$OUT_DIR/${name}.png"
done

echo
echo "Captured ${#PANELS[@]} panels."
echo "Note: Database / Docker are not yet bound to keyboard shortcuts."
echo "      Click them manually and re-run, or add Ctrl+8 / Ctrl+9 to"
echo "      app/shortcuts.rs for full coverage."
echo
open "$OUT_DIR"
