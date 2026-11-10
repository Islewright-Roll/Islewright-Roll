#!/usr/bin/env python3
"""Builds the sound effects in resource/assets/sfx (sfx_<id>.wav) from the sources named
in tools/sfx-sources.txt. Run it through tools/process_sfx.sh; see that file for usage.

For every sound, chosen and alternate alike:
  1. decode to mono 44.1 kHz float through GENTLE_LOWPASS (a first order low pass at 5 kHz: minus
     3 dB at 5 kHz, 6 dB per octave above, so the crackle a phone speaker makes of the high end is
     tamed without dulling the sound), then the table's own extra ffmpeg filter where it names one;
     the filter runs before the cut and the measuring, so loudness is of the sound as it ships;
     cut to the table's start and length;
  2. trim the leading silence to 2 ms and the trailing silence, then a 5 ms fade in and the table's
     fade out (raised cosine), so nothing clicks;
  3. measure loudness (see below), and apply one plain gain so that it sits at
     REFERENCE + MASTER_DB + the moment's offset. No compressor and no limiter: where the gain would push the
     sample peak above PEAK_CEILING_DB + MASTER_DB (lowered further by the moment's offset when that is
     negative, so a tick meant to be quiet is quiet in peak too) it is reduced instead, and the report says so.
  4. write 16 bit mono 44.1 kHz: WAV for sounds up to WAV_MAX_S long, Ogg Vorbis above that.

Loudness measure, used for every sound: "L100", the K-weighted (ITU-R BS.1770 pre-filter) mean
square of the loudest 100 ms window, in dB, with the BS.1770 -0.691 offset so it reads like LUFS.
Integrated LUFS is unreliable below about a second (the gating block is 400 ms, and ffmpeg's
ebur128 reports -70 for a click), so it is only quoted in the report where it is valid; peak is the
sample peak. REFERENCE is the median L100 of the chosen sounds that came from Wesnoth.

Outputs are deterministic: no dither, no random Ogg serial (bitexact).
"""
import hashlib
import os
import subprocess
import sys
import tempfile
import wave

import numpy as np

SR = 44100
LEAD_KEEP_MS = 2.0          # silence kept before the first real sample
LEAD_THRESHOLD_DB = -40.0   # "silence" is below this, relative to the sound's own peak
TAIL_THRESHOLD_DB = -50.0
TAIL_KEEP_MS = 10.0
FADE_IN_MS = 5.0
PEAK_CEILING_DB = -1.5      # headroom for the Vorbis encoder's overshoot
WAV_MAX_S = 0.35            # up to this long: WAV; longer: Ogg Vorbis
VORBIS_QUALITY = "6"
GENTLE_LOWPASS = "lowpass=f=5000:p=1"   # applied to every source, before its own filter
# The Wesnoth sounds whose median loudness is the reference every moment is levelled to
REFERENCE_KEYS = ("wes_button", "wes_slider", "wes_hatchet", "wes_warhammer", "wes_gold_one", "wes_leave", "wes_horn1", "wes_victory", "wes_join")
MASTER_DB = -4.0            # the whole set sits this much under the Wesnoth reference

HERE = os.path.dirname(os.path.abspath(__file__))
RAW = os.path.join(HERE, "..", "resource", "assets", "sfx")
TABLE = os.path.join(HERE, "sfx-sources.txt")


def fail(message):
    sys.stderr.write("process_sfx: " + message + "\n")
    sys.exit(1)


def read_table():
    moments, sources = [], {}
    section = None
    for line in open(TABLE, encoding="utf-8"):
        line = line.rstrip("\n")
        if not line.strip() or line.startswith("#"):
            continue
        if line.startswith("["):
            section = line.strip()
            continue
        f = line.split("|")
        if section == "[moments]":
            if len(f) != 6:
                fail("bad [moments] row: " + line)
            moments.append(dict(n=f[0], id=f[1], chosen=f[2], alt=f[3], offset=float(f[4]), intent=f[5]))
        elif section == "[sources]":
            if len(f) != 7:
                fail("bad [sources] row: " + line)
            sources[f[0]] = dict(key=f[0], spec=f[1], start=float(f[2]),
                                 length=None if f[3] == "-" else float(f[3]),
                                 fade_ms=float(f[4]), filt=None if f[5] == "-" else f[5], sha=f[6])
    for m in moments:
        for k in (m["chosen"], m["alt"]):
            if k not in sources:
                fail("moment %s names the unknown source %s" % (m["id"], k))
    return moments, sources


def locate(spec, originals, wesnoth):
    kind, _, path = spec.partition(":")
    base = {"wesnoth": wesnoth, "orig": originals}.get(kind)
    if base is None:
        fail("unknown source kind in " + spec)
    full = os.path.join(base, path)
    if not os.path.isfile(full):
        fail("missing original: " + full)
    return full


def sha256(path):
    h = hashlib.sha256()
    with open(path, "rb") as fh:
        h.update(fh.read())
    return h.hexdigest()


def decode(path, filt=None, gentle=True):
    af = (GENTLE_LOWPASS + "," if gentle else "") + (filt + "," if filt else "") + "aresample=%d" % SR
    r = subprocess.run(["ffmpeg", "-nostdin", "-v", "error", "-i", path, "-af", af, "-ac", "1",
                        "-f", "f32le", "-"], capture_output=True)
    if r.returncode != 0:
        fail("ffmpeg could not decode %s: %s" % (path, r.stderr.decode()))
    return np.frombuffer(r.stdout, dtype=np.float32).astype(np.float64)


def duration(path):
    r = subprocess.run(["ffprobe", "-v", "error", "-show_entries", "format=duration", "-of", "csv=p=0", path],
                       capture_output=True, text=True)
    return float(r.stdout.strip())


# ---- loudness -----------------------------------------------------------------------------------

def _biquad(x, b, a):
    try:
        from scipy.signal import lfilter
        return lfilter(b, a, x)
    except ImportError:
        y = np.zeros_like(x)
        x1 = x2 = y1 = y2 = 0.0
        for i, v in enumerate(x):
            o = b[0] * v + b[1] * x1 + b[2] * x2 - a[1] * y1 - a[2] * y2
            x2, x1 = x1, v
            y2, y1 = y1, o
            y[i] = o
        return y


def k_weight(x):
    # Stage 1: high shelf (BS.1770 values, derived for 44.1 kHz)
    f0, g, q = 1681.974450955533, 3.999843853973347, 0.7071752369554196
    A = 10 ** (g / 40)
    w0 = 2 * np.pi * f0 / SR
    al = np.sin(w0) / (2 * q)
    c = np.cos(w0)
    b = np.array([A * ((A + 1) + (A - 1) * c + 2 * np.sqrt(A) * al),
                  -2 * A * ((A - 1) + (A + 1) * c),
                  A * ((A + 1) + (A - 1) * c - 2 * np.sqrt(A) * al)])
    a = np.array([(A + 1) - (A - 1) * c + 2 * np.sqrt(A) * al,
                  2 * ((A - 1) - (A + 1) * c),
                  (A + 1) - (A - 1) * c - 2 * np.sqrt(A) * al])
    x = _biquad(x, b / a[0], a / a[0])
    # Stage 2: high pass
    f0, q = 38.13547087602444, 0.5003270373238773
    K = np.tan(np.pi * f0 / SR)
    a0 = 1 + K / q + K * K
    b = np.array([1.0, -2.0, 1.0])
    a = np.array([1.0, 2 * (K * K - 1) / a0, (1 - K / q + K * K) / a0])
    return _biquad(x, b / a0, a)


def l100(x):
    """Loudest 100 ms (hop 10 ms), K-weighted, dB. A sound shorter than 100 ms is averaged over 100 ms."""
    w = int(round(0.1 * SR))
    hop = int(round(0.01 * SR))
    k = k_weight(x)
    sq = np.concatenate([k * k, np.zeros(w)])
    c = np.concatenate([[0.0], np.cumsum(sq)])
    best = 0.0
    for i in range(0, max(1, len(x) - 1), hop):
        best = max(best, (c[i + w] - c[i]) / w)
    return 10 * np.log10(best + 1e-12) - 0.691


def peak_db(x):
    return 20 * np.log10(np.abs(x).max() + 1e-12)


def integrated_lufs(path):
    """ffmpeg ebur128 integrated loudness of a file, or None below its gate (very short sounds)."""
    r = subprocess.run(["ffmpeg", "-nostdin", "-i", path, "-af", "ebur128", "-f", "null", "-"],
                       capture_output=True, text=True)
    t = r.stderr
    i = t.rfind("Summary:")
    for line in t[i:].splitlines():
        line = line.strip()
        if line.startswith("I:"):
            v = line.split()[1]
            return None if v.startswith("-70") or v == "-inf" else float(v)
    return None


# ---- shaping ------------------------------------------------------------------------------------

def shape(x, src):
    s = int(round(src["start"] * SR))
    e = len(x) if src["length"] is None else min(len(x), s + int(round(src["length"] * SR)))
    x = x[s:e].copy()
    pk = np.abs(x).max()
    if pk == 0:
        fail(src["key"] + " is silent")
    lead = int(np.argmax(np.abs(x) >= pk * 10 ** (LEAD_THRESHOLD_DB / 20)))
    last = len(x) - 1 - int(np.argmax(np.abs(x[::-1]) >= pk * 10 ** (TAIL_THRESHOLD_DB / 20)))
    a = max(0, lead - int(round(LEAD_KEEP_MS / 1000 * SR)))
    b = min(len(x), last + 1 + int(round(TAIL_KEEP_MS / 1000 * SR)))
    x = x[a:b]
    n_in = min(len(x), int(round(FADE_IN_MS / 1000 * SR)))
    if n_in > 1:
        x[:n_in] *= np.sin(np.linspace(0, np.pi / 2, n_in)) ** 2
    n_out = min(len(x), int(round(src["fade_ms"] / 1000 * SR)))
    if n_out > 1:
        x[-n_out:] *= np.cos(np.linspace(0, np.pi / 2, n_out)) ** 2
    return x


def to_pcm16(x):
    return np.clip(np.rint(x * 32767.0), -32768, 32767).astype("<i2")


def write_wav(path, pcm):
    with wave.open(path, "wb") as w:
        w.setnchannels(1)
        w.setsampwidth(2)
        w.setframerate(SR)
        w.writeframes(pcm.tobytes())


def encode_ogg(pcm, out):
    with tempfile.TemporaryDirectory() as d:
        tmp = os.path.join(d, "in.wav")
        write_wav(tmp, pcm)
        r = subprocess.run(["ffmpeg", "-nostdin", "-v", "error", "-y", "-i", tmp, "-map_metadata", "-1",
                            "-fflags", "+bitexact", "-flags:a", "+bitexact", "-c:a", "libvorbis",
                            "-q:a", VORBIS_QUALITY, "-ac", "1", out], capture_output=True)
        if r.returncode != 0:
            fail("ffmpeg could not encode %s: %s" % (out, r.stderr.decode()))


def read_back(path):
    return decode(path, gentle=False)   # a shipped file is already filtered


def main():
    if len(sys.argv) == 3 and sys.argv[1] == "--index":
        # Rebuild only the audition page, from the WAV files already in that folder
        moments, sources = read_table()
        write_index(sys.argv[2], moments, sources)
        return
    if len(sys.argv) < 2:
        fail("usage: process_sfx.py <originals dir> [<audition dir>]")
    originals = sys.argv[1]
    audition = sys.argv[2] if len(sys.argv) > 2 else None
    wesnoth = os.environ.get("WESNOTH_DIR", "/usr/share/games/wesnoth/1.16")
    moments, sources = read_table()

    # decode and shape every source in use, checking its hash
    shaped, info = {}, {}
    for m in moments:
        for role in ("chosen", "alt"):
            key = m[role]
            if key in shaped:
                continue
            src = sources[key]
            path = locate(src["spec"], originals, wesnoth)
            got = sha256(path)
            if got != src["sha"]:
                fail("%s: sha256 is %s, the table says %s" % (path, got, src["sha"]))
            x = shape(decode(path, src["filt"]), src)
            shaped[key] = x
            info[key] = dict(orig_s=duration(path), l100=l100(x), peak=peak_db(x))

    # The reference is pinned to a fixed set of Wesnoth sounds, used or not, so that swapping
    # one moment's sound never moves the level of all the others
    for key in REFERENCE_KEYS:
        if key not in info:
            src = sources[key]
            path = locate(src["spec"], originals, wesnoth)
            if sha256(path) != src["sha"]:
                fail("%s: sha256 does not match the table" % path)
            info[key] = dict(l100=l100(shape(decode(path, src["filt"]), src)))
    ref_keys = sorted(info[key]["l100"] for key in REFERENCE_KEYS)
    reference = float(np.median(ref_keys))

    os.makedirs(RAW, exist_ok=True)
    for f in os.listdir(RAW):
        if f.startswith("sfx_") and f.endswith((".ogg", ".wav")):
            os.remove(os.path.join(RAW, f))
    if audition:
        os.makedirs(audition, exist_ok=True)
        for f in os.listdir(audition):
            if f[:2].isdigit() and f.endswith(".wav"):
                os.remove(os.path.join(audition, f))

    rows = []
    total = 0
    for m in moments:
        for role in ("chosen", "alt"):
            key = m[role]
            x = shaped[key]
            i = info[key]
            want = reference + MASTER_DB + m["offset"]
            gain = want - i["l100"]
            limited = False
            # A negative offset lowers the peak ceiling too, so a click meant to be quieter than a
            # build is quieter in peak as well as in loudness, even where its short length means
            # the loudness measure alone would let it reach the ceiling.
            ceiling = PEAK_CEILING_DB + MASTER_DB + min(0.0, m["offset"])
            if i["peak"] + gain > ceiling:
                gain = ceiling - i["peak"]
                limited = True
            y = x * 10 ** (gain / 20)
            pcm = to_pcm16(y)
            dur = len(pcm) / SR
            if role == "chosen":
                # Day's sound part plays WAV only (16-bit mono 44.1 kHz plays everywhere as it
                # is), so every clip ships as WAV now, however long it is.
                ext = "wav"
                dest = os.path.join(RAW, "sfx_%s.%s" % (m["id"], ext))
                final_path = dest
            else:
                ext = "wav"
                final_path = None
            if role == "chosen":
                if ext == "wav":
                    write_wav(dest, pcm)
                else:
                    encode_ogg(pcm, dest)
                size = os.path.getsize(dest)
                total += size
                final = read_back(dest)
            else:
                final = pcm.astype(np.float64) / 32768.0
                size = 0
            if audition:
                name = "%s_%s_%s.wav" % (m["n"], m["id"], "chosen" if role == "chosen" else "alt")
                if role == "chosen" and ext == "ogg":
                    # what ships: the Ogg, decoded
                    write_wav(os.path.join(audition, name), to_pcm16(final))
                else:
                    write_wav(os.path.join(audition, name), pcm)
            rows.append(dict(n=m["n"], id=m["id"], role=role, key=key, orig_s=i["orig_s"], dur=dur,
                             before=i["l100"], pk_before=i["peak"], gain=gain, after=l100(final),
                             pk_after=peak_db(final), want=want, limited=limited, ext=ext, size=size,
                             lufs=integrated_lufs(dest) if role == "chosen" else None))

    lines = []
    lines.append("reference (median L100 of the %d Wesnoth reference sounds) = %.2f dB" % (len(ref_keys), reference))
    lines.append("L100 = K-weighted loudest 100 ms, dB. peak = sample peak dBFS. lufs = ffmpeg integrated, '-' where its gate cannot read the sound.")
    lines.append("%-3s %-13s %-7s %-14s %6s %6s  %7s %7s  %6s  %7s %7s %6s %s" % (
        "n", "moment", "role", "source", "orig_s", "final_s", "L100_in", "pk_in", "gain", "L100_out", "pk_out", "lufs", "note"))
    for r in rows:
        note = []
        if r["limited"]:
            note.append("peak-limited (wanted %.1f)" % r["want"])
        if r["role"] == "chosen":
            note.append("%s %d B" % (r["ext"], r["size"]))
        lines.append("%-3s %-13s %-7s %-14s %6.2f %6.2f  %7.1f %7.1f  %+6.1f  %7.1f %7.1f %6s %s" % (
            r["n"], r["id"], r["role"], r["key"], r["orig_s"], r["dur"], r["before"], r["pk_before"], r["gain"],
            r["after"], r["pk_after"], "-" if r["lufs"] is None else "%.1f" % r["lufs"], "; ".join(note)))
    lines.append("total shipped: %d bytes" % total)
    report = "\n".join(lines) + "\n"
    sys.stdout.write(report)
    if audition:
        with open(os.path.join(audition, "report.txt"), "w") as fh:
            fh.write(report)
        write_index(audition, moments, sources)


# The sprite shown beside each moment on the audition page: a file in resource/assets/art
INDEX_SPRITES = {
    "click": "ui_button_normal.png", "die": "die_face_held.png", "roll": "die_face.png", "roll_b": "die_face.png",
    "road": "road_h.png", "village": "piece_village.png", "castle": "piece_castle.png",
    "trader": "piece_trader.png", "banked": "icon_any_16.png", "gold": "icon_gold_16.png",
    "turn_end": "track_flag.png", "empty_turn": "track_flag_empty.png",
    "ship_appears": "piece_crownship.png", "ship_docks": "track_pier.png",
    "game_over": "track_ship.png", "top_score": "glyph_castle_12.png", "bonus": "glyph_trader_12.png",
}


def data_uri(path, mime):
    import base64
    with open(path, "rb") as fh:
        return "data:%s;base64,%s" % (mime, base64.b64encode(fh.read()).decode("ascii"))


def write_index(audition, moments, sources):
    """One self-contained page: every sound and sprite is embedded, so it plays wherever it is
    opened (a browser, a preview pane) with no files beside it."""
    art = os.path.join(HERE, "..", "resource", "assets", "art")

    def desc(key):
        s = sources[key]
        return s["spec"].replace("wesnoth:", "Wesnoth 1.16: ").replace("orig:", "")

    def player(name):
        path = os.path.join(audition, name)
        if not os.path.exists(path):
            return "<i>missing %s</i>" % name
        return "<audio controls preload=auto src='%s'></audio>" % data_uri(path, "audio/wav")

    out = ["<!doctype html><meta charset=utf-8><title>Islewright sound effects</title>",
           "<style>body{font:15px system-ui;margin:2em;max-width:64em;background:#2e2216;color:#d2be94}"
           "td{padding:.5em .8em;vertical-align:middle;border-top:1px solid #5a4630}audio{width:250px}"
           "code{font-size:12px;color:#e0b870}img{image-rendering:pixelated;height:64px}"
           "td.s{text-align:center;width:120px}a{color:#7db8d8}</style>",
           "<h1>Islewright sound effects</h1>",
           "<p>Every sound is shown as it ships (loudness matched). Change a sound by editing the key in "
           "<code>tools/sfx-sources.txt</code> and running <code>tools/process_sfx.sh</code>. To rebuild only "
           "this page from the WAV files already here: <code>python3 -I tools/process_sfx.py --index "
           "&lt;this folder&gt;</code>.</p>",
           "<table><tr><th><th>moment<th>chosen<th>alternate"]
    for m in moments:
        n, i = m["n"], m["id"]
        sprite = os.path.join(art, INDEX_SPRITES.get(i, ""))
        img = "<img alt='' src='%s'>" % data_uri(sprite, "image/png") if os.path.isfile(sprite) else ""
        out.append("<tr><td class=s>%s<td><b>%s %s</b><br>%s" % (img, n, i, m["intent"]))
        for role in ("chosen", "alt"):
            key = m[role]
            out.append("<td>%s<br><code>%s</code><br>%s" % (player("%s_%s_%s.wav" % (n, i, role)), key, desc(key)))
    extra = sorted(f for f in os.listdir(audition) if f.endswith(".wav") and not f[:2].isdigit())
    if extra:
        out.append("</table><h2>Other candidates</h2><table>")
        for f in extra:
            out.append("<tr><td><code>%s</code><td>%s" % (f, player(f)))
    out.append("</table><p>Sources and licences: see the end of tools/sfx-sources.txt.</p>")
    with open(os.path.join(audition, "index.html"), "w", encoding="utf-8") as fh:
        fh.write("\n".join(out) + "\n")


if __name__ == "__main__":
    main()
