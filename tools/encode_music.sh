#!/usr/bin/env bash
# Builds the soundtrack files in resource/assets/sfx from the artist's original downloads.
# It does not download anything: put the originals in one directory and pass it in.
#
#   tools/encode_music.sh /path/to/originals
#
# The ten tracks are by Vindsvept, CC BY 4.0 (see tools/music-sources.txt). An original is
# matched to a track by its title in the file name (case, punctuation and the album prefix
# Bandcamp adds do not matter), and the result is named music_<slug>.<ext>.
#
#   * A LOSSLESS original (.flac or .wav) is encoded to $FORMAT, below.
#   * A LOSSY original (.mp3, .ogg, .opus, .m4a) is copied byte for byte, never re-encoded:
#     lossy to lossy only loses quality. Its extension is kept.
#
# To change the shipped format of the lossless tracks, edit FORMAT or the bitrate/quality here and
# run this again. Any older music_<slug>.* is removed first, so a changed extension leaves
# no duplicate. The app finds the tracks by the music_ prefix, whatever the extension.

# opus = Opus (libopus, VBR, stereo) in an Ogg container at OPUS_BITRATE, the chosen format;
# vorbis = Ogg Vorbis at VORBIS_QUALITY; flac = FLAC, compression level 8 (lossless, much larger).
# All three are written as music_<slug>.ogg (.flac for FLAC): Android's resource packaging
# and MediaPlayer both take .ogg for Ogg Opus.
FORMAT=opus
OPUS_BITRATE=192k
# libvorbis -q:a: 6 is about 192 kbps, 8 about 256, 9 about 320
VORBIS_QUALITY=8

set -euo pipefail

if [ $# -ne 1 ] || [ ! -d "$1" ]; then
    echo "usage: $0 <directory of the original downloads>" >&2
    exit 2
fi
SRC=$1
HERE=$(cd "$(dirname "$0")" && pwd)
OUT="$HERE/../resource/assets/music"
mkdir -p "$OUT"

# slug|text to find in the normalised file name (lowercase, apostrophes dropped, other
# punctuation as _). Into the Mind's Eye has two parts; only part one is used.
TRACKS='deep_sleep_of_the_elven_king|deep_sleep_of_the_elven_king
deeper_into_the_woods|deeper_into_the_woods
wanderlust|wanderlust
lady_of_the_lake|lady_of_the_lake
what_lies_beyond|what_lies_beyond
into_the_minds_eye|into_the_minds_eye_part_one
breath_of_life|breath_of_life
to_the_skies|to_the_skies
winters_night|winters_night
the_old_way|the_old_way'

normalise() {
    printf '%s' "$1" | tr 'A-Z' 'a-z' | sed "s/[’'\`]//g; s/[^a-z0-9]\{1,\}/_/g"
}

# The original in $SRC whose normalised name contains $1 (a lossless one preferred)
find_original() {
    local want=$1 best="" f base norm
    for f in "$SRC"/*; do
        [ -f "$f" ] || continue
        base=$(basename "$f")
        norm=$(normalise "${base%.*}")
        case "_${norm}_" in
            *"_${want}_"*)
                case "${f##*.}" in
                    flac|wav|FLAC|WAV) echo "$f"; return ;;
                    *) [ -z "$best" ] && best=$f ;;
                esac ;;
        esac
    done
    if [ -n "$best" ]; then
        echo "$best"
    fi
    return 0
}

missing=0
while IFS='|' read -r slug want; do
    original=$(find_original "$want")
    if [ -z "$original" ]; then
        echo "MISSING  $slug (no file with \"$want\" in its name in $SRC)" >&2
        missing=$((missing + 1))
        continue
    fi
    ext=$(printf '%s' "${original##*.}" | tr 'A-Z' 'a-z')
    case "$ext" in
        flac|wav)
            rm -f "$OUT"/music_"$slug".*
            case "$FORMAT" in
                opus)
                    dest="$OUT/music_$slug.ogg"
                    # -vn: no cover art. -map_metadata 0 keeps the title and artist tags.
                    # Opus always runs at 48 kHz, so a 44.1 kHz original is resampled by libopus.
                    ffmpeg -nostdin -loglevel error -y -i "$original" -vn -map_metadata 0 \
                        -c:a libopus -b:a "$OPUS_BITRATE" -vbr on -application audio -ac 2 "$dest"
                    ;;
                vorbis)
                    dest="$OUT/music_$slug.ogg"
                    # The sample rate and channels are the original's own.
                    ffmpeg -nostdin -loglevel error -y -i "$original" -vn -map_metadata 0 \
                        -c:a libvorbis -q:a "$VORBIS_QUALITY" "$dest"
                    ;;
                flac)
                    dest="$OUT/music_$slug.flac"
                    ffmpeg -nostdin -loglevel error -y -i "$original" -vn -map_metadata 0 \
                        -c:a flac -compression_level 8 "$dest"
                    ;;
                *) echo "FORMAT must be opus, vorbis or flac, not $FORMAT" >&2; exit 2 ;;
            esac
            how="encoded from $ext"
            ;;
        mp3|ogg|opus|m4a)
            rm -f "$OUT"/music_"$slug".*
            dest="$OUT/music_$slug.$ext"
            cp -- "$original" "$dest"
            how="copied unmodified ($ext, lossy)"
            ;;
        *)
            echo "SKIPPED  $slug: $original is not an audio format this script knows" >&2
            missing=$((missing + 1))
            continue
            ;;
    esac
    dur=$(ffprobe -v error -show_entries format=duration -of csv=p=0 "$dest")
    printf '%-26s %s  %7.1f s  %9d bytes  sha256(original) %s\n' "music_$slug" "$how" "$dur" \
        "$(stat -c %s "$dest")" "$(sha256sum "$original" | cut -d' ' -f1)"
done <<< "$TRACKS"

echo
du -ch "$OUT"/music_* 2>/dev/null | tail -1
if [ "$missing" -ne 0 ]; then
    echo "$missing track(s) missing or skipped" >&2
    exit 1
fi
