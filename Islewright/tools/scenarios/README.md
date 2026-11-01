# Debug scenarios

Each JSON file puts one island into a known state on a debug build. The schema is
documented at the top of `app/src/debug/java/com/ridgelineapps/resdicegame/DebugScenario.java`.

Push one to the app and apply it:

    adb push tools/scenarios/dunholm-nine-points.json \
      /sdcard/Android/data/com.ridgelineapps.resdicegame/files/debug-scenario.json

Then open the in-game Menu and choose Debug Scenario. A toast says whether it applied.

If a scenario's `"map"` differs from the island on screen, the loader switches island first
(the other island keeps its own saved game) and then applies it.

The HUD strip's per turn flags read the running totals the pieces never earned in a scenario, so
the loader fills them: the score rises evenly across the `turnsTaken` passed turns and ends on the
scenario's score, leaving the turn in progress at +0. To shape it, add `"turnScores": [2, 3, 1]`,
the points gained in each passed turn (exactly `turnsTaken` whole numbers, 0 or more); the turn in
progress then shows whatever the score has over their sum.

Files: `<map>-fresh`, `-midgame`, `-full`, plus `tarnholm-final-turn`, `dunholm-last-turn`
(turn 20 of 20) and `dunholm-nine-points` (one village short of the 10 point target, with
dice that can build it, to test the instant finish).

Hiring animation and confirmation: `tarnholm-hire-next` (two traders hired, dice that pay for the third,
third roll so no early build question) and `dunholm-pair-half-hired` (trader 6 of the north desert's pair
hired, dice that pay for any trader: hire 7 to complete the pair and play the whole animation from the
shared "?", hire 8 or 9 to get the "Start a second pair?" question). Both are on the third roll.
