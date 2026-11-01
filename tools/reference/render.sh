#!/usr/bin/env bash
# Render tools/reference/<map>.map with Wesnoth and compare it with the bake.
#
#   tools/reference/render.sh [MAP] [--data-dir DIR] [--bake PNG] [--tolerance N]
#
# MAP is the map id: tarnholm (the default) or dunholm.
#
# Environment:
#   WESNOTH_BIN   the wesnoth binary (default: wesnoth)
#
# Options:
#   --data-dir DIR   run Wesnoth with another data directory (for example the sister
#                    checkout at ../wesnoth) instead of the package's own
#   --bake PNG       compare against another bake (default: the shipped board_<MAP>.png); make
#                    one with BAKE_OUT=/tmp/x.png BAKE_FOREST_IN_HEX=0 python3 -I tools/bake_board.py MAP
#   --tolerance N    a pixel counts as different when its largest channel differs by more
#                    than N (default 16)
#
# Writes tools/reference/<MAP>_wesnoth_render.png, <MAP>_side_by_side.png (bake, Wesnoth,
# difference; 2x) and <MAP>_diff_stats.txt. Wesnoth runs with a throwaway user data directory so nothing is
# read from or written to your own Wesnoth settings (the board is only correct at the
# default 72 px zoom, and your preferences could change it).
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo="$(cd "$here/../.." && pwd)"
bin="${WESNOTH_BIN:-wesnoth}"
map_id="tarnholm"
data_dir=""
bake=""
tolerance=16

while [ $# -gt 0 ]; do
    case "$1" in
        --data-dir) data_dir="${2:?--data-dir needs a directory}"; shift 2 ;;
        --bake) bake="${2:?--bake needs a png}"; shift 2 ;;
        --tolerance) tolerance="${2:?--tolerance needs a number}"; shift 2 ;;
        -h|--help) sed -n '2,22p' "$0"; exit 0 ;;
        -*) echo "render.sh: unknown argument: $1" >&2; exit 2 ;;
        *) map_id="$1"; shift ;;
    esac
done

map_file="$here/$map_id.map"
if [ ! -f "$map_file" ]; then
    echo "render.sh: no map file $map_file (maps: $(cd "$here" && ls *.map | sed 's/\.map$//' | tr '\n' ' '))" >&2
    exit 2
fi
if [ -z "$bake" ]; then
    bake="$repo/app/src/main/res/drawable-nodpi/board_$map_id.png"
fi

if ! command -v "$bin" >/dev/null 2>&1; then
    echo "render.sh: wesnoth not found (looked for '$bin')." >&2
    echo "Install the Wesnoth package (Debian and Ubuntu: sudo apt install wesnoth) or set WESNOTH_BIN to the binary." >&2
    exit 1
fi
if [ ! -f "$bake" ]; then
    echo "render.sh: bake image not found: $bake (run python3 -I tools/bake_board.py first)" >&2
    exit 1
fi
if [ -n "$data_dir" ]; then
    data_dir="$(cd "$data_dir" && pwd)"
fi

work="$(mktemp -d "${TMPDIR:-/tmp}/board-render.XXXXXX")"
trap 'rm -rf "$work"' EXIT
mkdir -p "$work/userdata" "$work/userconfig"

args=()
if [ -n "$data_dir" ]; then
    args+=(--data-dir "$data_dir")
fi
args+=(--userdata-dir "$work/userdata" --userconfig-dir "$work/userconfig"
       --screenshot "$map_file" "$work/shot.png")

# --screenshot opens a hidden window, so it needs a display. Use a virtual one when
# there is none and xvfb-run is installed.
runner=()
if [ -z "${DISPLAY:-}" ] && [ -z "${WAYLAND_DISPLAY:-}" ] && command -v xvfb-run >/dev/null 2>&1; then
    runner=(xvfb-run -a)
fi

echo "render.sh: running $bin --screenshot ..."
if ! "${runner[@]}" "$bin" "${args[@]}" >"$work/wesnoth.log" 2>&1; then
    echo "render.sh: wesnoth failed; last log lines:" >&2
    tail -n 15 "$work/wesnoth.log" >&2
    exit 1
fi
if [ ! -s "$work/shot.png" ]; then
    echo "render.sh: wesnoth produced no screenshot; last log lines:" >&2
    tail -n 15 "$work/wesnoth.log" >&2
    exit 1
fi

python3 -I "$here/compare.py" "$work/shot.png" "$bake" "$here" "$map_id" "$tolerance"
echo "render.sh: wrote $here/${map_id}_wesnoth_render.png and $here/${map_id}_side_by_side.png"
