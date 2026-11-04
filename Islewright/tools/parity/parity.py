#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
"""Closed-loop visual comparison of the Java app (v2) and the Day port (v3) on one emulator.

Both debug builds are driven into the same screens through their debug scenario harnesses, and
each screen is captured from the device itself, so what is compared is what a player sees.

    parity.py capture v2 [screen ...]     capture to build/parity/v2/<screen>.png
    parity.py capture v3 [screen ...]     capture to build/parity/v3/<screen>.png
    parity.py compare [screen ...]        write diffs and report.txt, print the scores
    parity.py list                        name the screens

The score is the share of pixels whose channels are all within TOLERANCE of v2's. A screen
counts as matched at 95. Installing the two debug APKs on the emulator is left to the caller.
"""

import json
import os
import subprocess
import sys
import time

from PIL import Image, ImageChops

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "..", ".."))
OUT = os.path.join(ROOT, "build", "parity")
SCENARIOS = os.path.join(ROOT, "Islewright", "tools", "scenarios")
SERIAL = os.environ.get("PARITY_SERIAL", "emulator-5554")
TOLERANCE = 10

# Two things have no score. The Java app holds portrait, so the turned-* screens show the port's
# turned layouts on their own. And the Java app ignores the system text size, where the port
# follows it, so the large-text-* screens are for reading rather than for a number.

V2_PKG = "io.github.islewright_roll.islewright"
V2_ACTIVITY = V2_PKG + "/com.ridgelineapps.resdicegame.MainActivity"
V2_FILES = f"/sdcard/Android/data/{V2_PKG}/files"
V3_PKG = "io.github.islewrightroll.islewright.dev"
V3_ACTIVITY = V3_PKG + "/dev.daybrite.day.bridge.DayActivity"

EARLY_PROMPT_SMART = 5

# Where things are on the 1080 x 2400 screen, in device pixels. A debug build of either app lists
# Debug Scenario in its menu, so the two menus are the same height and their entries line up.
MENU_BUTTON = (990, 115)
ROLL = (536, 2198)
ROAD_SPOT = (390, 1120)
MENU_ENTRY = {"restart": 778, "island": 908, "settings": 1038, "rules": 1168, "scores": 1298, "about": 1428, "debug": 1558}

# screen name: (scenario file or None, steps after the scenario)
SCREENS = {
    "tarnholm-fresh": ("tarnholm-fresh", []),
    "tarnholm-midgame": ("tarnholm-midgame", []),
    "tarnholm-full": ("tarnholm-full", []),
    "tarnholm-final-turn": ("tarnholm-final-turn", []),
    "dunholm-fresh": ("dunholm-fresh", []),
    "dunholm-midgame": ("dunholm-midgame", []),
    "dunholm-full": ("dunholm-full", []),
    "dunholm-nine-points": ("dunholm-nine-points", []),
    "dunholm-last-turn": ("dunholm-last-turn", []),
    "menu": ("tarnholm-fresh", [("tap", MENU_BUTTON)]),
    "restart": ("tarnholm-fresh", [("tap", MENU_BUTTON), ("menu", "restart")]),
    "settings": ("tarnholm-fresh", [("tap", MENU_BUTTON), ("menu", "settings")]),
    "rules": ("tarnholm-fresh", [("tap", MENU_BUTTON), ("menu", "rules")]),
    "scores": ("tarnholm-fresh", [("tap", MENU_BUTTON), ("menu", "scores")]),
    "about": ("tarnholm-fresh", [("tap", MENU_BUTTON), ("menu", "about")]),
    "welcome": (None, []),
    "settings-middle": ("tarnholm-fresh", [("tap", MENU_BUTTON), ("menu", "settings"), ("scroll", (1500, 400, 1))]),
    "settings-lower": ("tarnholm-fresh", [("tap", MENU_BUTTON), ("menu", "settings"), ("scroll", (1500, 400, 2))]),
    "rules-middle": ("tarnholm-fresh", [("tap", MENU_BUTTON), ("menu", "rules"), ("scroll", (1900, 300, 2))]),
    "rules-lower": ("tarnholm-fresh", [("tap", MENU_BUTTON), ("menu", "rules"), ("scroll", (1900, 300, 4))]),
    "about-end": ("tarnholm-fresh", [("tap", MENU_BUTTON), ("menu", "about"), ("scroll", (1900, 300, 3))]),
    "scores-dunholm": ("tarnholm-fresh", [("tap", MENU_BUTTON), ("menu", "scores"), ("tap", (640, 555))]),
    "tutorial": ("tutorial", []),
    "color-picker": ("tarnholm-fresh", [("tap", MENU_BUTTON), ("menu", "settings"), ("scroll", (1500, 400, 2)), ("tap", {"v2": (540, 1842), "v3": (540, 1220)})]),
    "hire-next": ("tarnholm-hire-next", []),
    "four-turns-left": ("tarnholm-four-turns-left", []),
    "trader-chain": ("tarnholm-trader-chain", []),
    "pair-half-hired": ("dunholm-pair-half-hired", []),
    "game-over": ("tarnholm-final-turn", [("tap", ROLL)]),
    "build-ready": ("tarnholm-build-ready", []),
    "end-turn-prompt": ("tarnholm-build-ready", [("tap", ROLL)]),
    "last-build": ("tarnholm-last-build", []),
    "early-build": ("tarnholm-build-early", [("tap", ROAD_SPOT)]),
    "road-built": ("tarnholm-build-ready", [("tap", ROAD_SPOT)]),
    "next-turn": ("tarnholm-build-ready", [("tap", ROLL), ("tap", (304, 1260))]),
    "last-built": ("tarnholm-last-build", [("tap", (570, 1700))]),
    "best-score": ("tarnholm-last-build", [("tap", (570, 1700)), ("tap", ROLL), ("tap", (304, 1260))]),
    "undo": ("tarnholm-build-ready", [("tap", ROAD_SPOT), ("tap", (1032, 2198))]),
    "early-prompt": ("tarnholm-build-prompted", [("tap", ROAD_SPOT)]),
    "hire-pair": ("dunholm-pair-half-hired", [("tap", (540, 880)), ("wait", 4)]),
    "second-pair-prompt": ("dunholm-pair-half-hired", [("tap", (540, 1090))]),
    "second-pair-hired": ("dunholm-pair-half-hired", [("tap", (540, 1090)), ("tap", (304, 1260)), ("wait", 4)]),
    "classic-bank": ("tarnholm-last-build", [("settings", {"traderClassic": True})]),
    "classic-drag": ("tarnholm-last-build", [("settings", {"traderClassic": True}), ("drag", (64, 1880, 110, 2050))]),
    "classic-dragging": ("tarnholm-last-build", [("settings", {"traderClassic": True}), ("hold-drag", (64, 1880, 110, 2050))]),
    "early-prompt-yes": ("tarnholm-build-prompted", [("tap", ROAD_SPOT), ("tap", (304, 1260))]),
    "go-to-dunholm": ("tarnholm-fresh", [("tap", MENU_BUTTON), ("menu", "island")]),
    "hold-roll": ("tarnholm-fresh", [("hold", ROLL)]),
    "hold-menu-entry": ("tarnholm-fresh", [("tap", MENU_BUTTON), ("hold", (540, 778))]),
    "hold-menu-button": ("tarnholm-fresh", [("hold", MENU_BUTTON)]),
    "turned-fresh": ("tarnholm-fresh", [("rotate", 1)]),
    "turned-midgame": ("dunholm-midgame", [("rotate", 1)]),
    "turned-full": ("tarnholm-full", [("rotate", 1)]),
    "turned-menu": ("tarnholm-fresh", [("tap", MENU_BUTTON), ("rotate", 1)]),
    "turned-rules": ("tarnholm-fresh", [("tap", MENU_BUTTON), ("menu", "rules"), ("rotate", 1)]),
    "turned-settings": ("tarnholm-fresh", [("tap", MENU_BUTTON), ("menu", "settings"), ("rotate", 1)]),
    "turned-back": ("tarnholm-fresh", [("rotate", 3)]),
    "large-text-fresh": ("tarnholm-fresh", [("font", "1.3")]),
    "large-text-prompt": ("tarnholm-build-prompted", [("font", "1.3"), ("tap", ROAD_SPOT)]),
    "large-text-rules": ("tarnholm-fresh", [("font", "1.3"), ("tap", MENU_BUTTON), ("menu", "rules")]),
    "night-fresh": ("tarnholm-fresh", [("night", True)]),
    "night-menu": ("tarnholm-fresh", [("night", True), ("tap", MENU_BUTTON)]),
    "night-settings": ("tarnholm-fresh", [("night", True), ("tap", MENU_BUTTON), ("menu", "settings")]),
    "night-rules": ("tarnholm-fresh", [("night", True), ("tap", MENU_BUTTON), ("menu", "rules")]),
    "night-scores": ("tarnholm-fresh", [("night", True), ("tap", MENU_BUTTON), ("menu", "scores")]),
    "night-welcome": (None, [("night", True)]),
}


def adb(*args, **kw):
    return subprocess.run(["adb", "-s", SERIAL, *args], check=True, capture_output=True, **kw).stdout


def tap(x, y, wait=0.9):
    adb("shell", "input", "tap", str(x), str(y))
    time.sleep(wait)


def shot(path):
    png = adb("exec-out", "screencap", "-p")
    os.makedirs(os.path.dirname(path), exist_ok=True)
    with open(path, "wb") as f:
        f.write(png)


def v2_settings(welcome, tutorial=False, extra=None):
    """The Java app keeps its settings in a JSON document a debug harness can write."""
    doc = {"schemaVersion": 1, "highScores": [], "settings": {"tutorial": tutorial, "showWelcome": welcome, **(extra or {})}}
    path = os.path.join(OUT, "v2-settings.json")
    with open(path, "w") as f:
        json.dump(doc, f)
    adb("shell", "mkdir", "-p", V2_FILES)
    adb("push", path, f"{V2_FILES}/highscores-and-settings.json")


def start_v2(scenario, extra=None):
    adb("shell", "am", "force-stop", V2_PKG)
    if scenario in (None, "tutorial"):
        adb("shell", "pm", "clear", V2_PKG)
    v2_settings(welcome=scenario is None, tutorial=scenario == "tutorial", extra=extra)
    if scenario == "tutorial":
        scenario = None
    if scenario:
        adb("push", os.path.join(SCENARIOS, scenario + ".json"), f"{V2_FILES}/debug-scenario.json")
    adb("shell", "am", "start", "-n", V2_ACTIVITY)
    time.sleep(2.5)
    if scenario:
        tap(*MENU_BUTTON)
        tap(540, MENU_ENTRY["debug"], wait=3.5)


def start_v3(scenario, extra=None):
    adb("shell", "am", "force-stop", V3_PKG)
    tutorial = scenario == "tutorial"
    extra = extra or {}
    args = ["shell", "am", "start", "-n", V3_ACTIVITY, "--es", "day.env.ISLEWRIGHT_TUTORIAL", "1" if tutorial else "0"]
    args += ["--es", "day.env.ISLEWRIGHT_EARLY_BUILD", str(extra.get("earlyBuildPrompt", EARLY_PROMPT_SMART))]
    args += ["--es", "day.env.ISLEWRIGHT_TRADER_CLASSIC", "1" if extra.get("traderClassic") else "0"]
    # Nothing a capture before this one saved, a high score say, may show in this one
    adb("shell", "pm", "clear", V3_PKG)
    if tutorial:
        args += ["--es", "day.env.ISLEWRIGHT_WELCOME", "0"]
    elif scenario:
        args += ["--es", "day.env.ISLEWRIGHT_WELCOME", "0"]
        with open(os.path.join(SCENARIOS, scenario + ".json")) as f:
            doc = json.dumps(json.load(f), separators=(",", ":")).replace('"', '\\"')
        args += ["--es", "day.env.ISLEWRIGHT_SCENARIO", f'"{doc}"']
    adb(*args)
    time.sleep(5)


def capture(version, names):
    for name in names:
        scenario, steps = SCREENS[name]
        # Night mode comes before the launch, as it does for a player who keeps it on
        if any(step == "night" for step, _ in steps):
            adb("shell", "cmd", "uimode", "night", "yes")
            time.sleep(1)
        extra = next((arg for step, arg in steps if step == "settings"), None)
        scale = next((arg for step, arg in steps if step == "font"), None)
        if scale:
            adb("shell", "settings", "put", "system", "font_scale", scale)
            time.sleep(1)
        (start_v2 if version == "v2" else start_v3)(scenario, extra)
        for step, arg in steps:
            if step == "tap":
                tap(*(arg[version] if isinstance(arg, dict) else arg))
            elif step == "menu":
                tap(540, MENU_ENTRY[arg], wait=1.5)
            elif step == "hold":
                # A finger left down: the shot comes while it is
                subprocess.Popen(["adb", "-s", SERIAL, "shell", "input", "swipe", str(arg[0]), str(arg[1]), str(arg[0]), str(arg[1]), "2500"])
                time.sleep(1.2)
            elif step == "rotate":
                # Turned by hand, once the screen is up: the taps above are all in portrait
                adb("shell", "settings", "put", "system", "accelerometer_rotation", "0")
                adb("shell", "settings", "put", "system", "user_rotation", str(arg))
                time.sleep(3)
            elif step in ("night", "settings", "font"):
                pass  # taken care of before the launch
            elif step == "drag":
                adb("shell", "input", "swipe", *map(str, arg), "700")
                time.sleep(1)
            elif step == "hold-drag":
                # Most of the way there, slowly enough that the shot comes in flight
                subprocess.Popen(["adb", "-s", SERIAL, "shell", "input", "swipe", *map(str, arg), "6000"])
                time.sleep(3)
            elif step == "wait":
                time.sleep(arg)
            elif step == "scroll":
                # A slow drag, so that neither platform flings: (from y, to y, times)
                y0, y1, times = arg
                for _ in range(times):
                    adb("shell", "input", "swipe", "540", str(y0), "540", str(y1), "1800")
                    time.sleep(0.6)
        shot(os.path.join(OUT, version, name + ".png"))
        if any(step == "night" for step, _ in steps):
            adb("shell", "cmd", "uimode", "night", "no")
        if any(step == "rotate" for step, _ in steps):
            adb("shell", "settings", "put", "system", "user_rotation", "0")
            time.sleep(1)
        if scale:
            adb("shell", "settings", "put", "system", "font_scale", "1.0")
        if any(step in ("hold", "hold-drag") for step, _ in steps):
            time.sleep(4)
        print("captured", version, name, flush=True)


def score(a, b):
    diff = ImageChops.difference(a, b).convert("RGB")
    data = diff.getdata()
    good = sum(1 for p in data if max(p) <= TOLERANCE)
    return 100.0 * good / (a.width * a.height), diff


def compare(names):
    rows = []
    os.makedirs(os.path.join(OUT, "diff"), exist_ok=True)
    for name in names:
        p2, p3 = (os.path.join(OUT, v, name + ".png") for v in ("v2", "v3"))
        if not (os.path.exists(p2) and os.path.exists(p3)):
            rows.append((name, None))
            continue
        a, b = Image.open(p2).convert("RGB"), Image.open(p3).convert("RGB")
        if a.size != b.size:
            rows.append((name, None))
            continue
        pct, diff = score(a, b)
        rows.append((name, pct))
        # v2, v3, and what differs (white is a wrong pixel), side by side
        mask = diff.convert("L").point(lambda v: 255 if v > TOLERANCE else 0).convert("RGB")
        side = Image.new("RGB", (a.width * 3 + 40, a.height), (255, 0, 255))
        side.paste(a, (0, 0))
        side.paste(b, (a.width + 20, 0))
        side.paste(mask, (2 * a.width + 40, 0))
        side.resize((side.width // 2, side.height // 2), Image.NEAREST).save(os.path.join(OUT, "diff", name + ".png"))
    lines = [f"{'n/a' if p is None else format(p, '6.2f')}  {n}" for n, p in rows]
    report = "\n".join(lines) + "\n"
    with open(os.path.join(OUT, "report.txt"), "w") as f:
        f.write(report)
    print(report, end="")


def main():
    cmd = sys.argv[1] if len(sys.argv) > 1 else "list"
    if cmd == "list":
        print("\n".join(SCREENS))
    elif cmd == "capture":
        capture(sys.argv[2], sys.argv[3:] or list(SCREENS))
    elif cmd == "compare":
        compare(sys.argv[2:] or list(SCREENS))
    else:
        sys.exit(__doc__)


if __name__ == "__main__":
    main()
