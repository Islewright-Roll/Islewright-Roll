#!/usr/bin/env bash
# Builds the sound effects in app/src/main/res/raw (sfx_<id>.ogg or .wav) from the original
# files. It does not download anything: put the originals in one directory and pass it in.
#
#   tools/process_sfx.sh /path/to/originals [/path/to/audition-dir]
#
# Which source plays which moment, and the per-moment loudness offsets, are in
# tools/sfx-sources.txt (a swap is a one-line change there plus a re-run of this). Wesnoth
# sounds are read from the installed package (WESNOTH_DIR, default
# /usr/share/games/wesnoth/1.16); everything else from the originals directory. The second
# argument, if given, also receives NN_moment_chosen.wav and NN_moment_alt.wav for listening,
# an index.html with players, and report.txt. The shaping and loudness method is described at the
# top of tools/process_sfx.py. Needs ffmpeg (with libvorbis), ffprobe and python3 with numpy;
# the output is byte-identical from one run to the next.
set -euo pipefail

if [ $# -lt 1 ] || [ ! -d "$1" ]; then
    echo "usage: $0 <directory of the original downloads> [<audition directory>]" >&2
    exit 2
fi
for tool in ffmpeg ffprobe python3; do
    command -v "$tool" >/dev/null || { echo "$tool is needed" >&2; exit 2; }
done
HERE=$(cd "$(dirname "$0")" && pwd)
exec python3 -I "$HERE/process_sfx.py" "$@"
