RFI-ID: RFI-RFC-009-READY
Created: 2026-08-09
Created-By: software_developer
Target-Role: code_reviewer
Related-Specification: SPEC-OUTPUT-RENDER-V1 / RFC-009
Priority: high
Status: Open

Subject: Is the Recorder mp4/race fix (RFC-009) ready to merge? Includes real empirical before/after verification, not just source review.

Context: RFC-009 (high severity: RENDER's downloaded `.mp4` reported
"format isn't supported" externally; a second recording produced a
0-byte file). Two pre-existing `Recorder` defects. Implemented on branch
`claude/dev-session-plw4fq`, scoped entirely to
`ui/scripts/engine/recorder.js`. Full detail in
`.agents/communication/implementation_reports/RFC009_recorder_mp4_fix_report.md`.

**This one comes with real test evidence, not just a trace** - this
sandbox turned out to have a working headless Chromium (previously noted
in `NOTIFICATION-002` as present-but-useless without a `wasm32` build).
This RFC's fix is pure JS, so I built a throwaway test harness (not
committed), imported the real `Recorder` into a real page, and drove two
back-to-back recording sessions with zero gap - deliberately worst-case
timing for the race.

**Result: reproduced the exact bug against the original code (5/5 runs),
and confirmed it's gone against the fix (5/5 runs).** Original code:
session 2's downloaded file was consistently ~3674 bytes - parsing the
raw MP4 box structure by hand shows it's literally **two complete MP4
files concatenated** (session 1's full stream, then session 2's full
stream) - the shared `this.chunks` race caught in the act. Fixed code:
session 2 is consistently exactly 1736 bytes, a single clean stream, no
concatenation, across all 5 runs.

**Fix 1's honest limitation:** this sandbox's Chromium has no H.264
encoder (`isTypeSupported` returns `false` for both `avc1` codec
strings), so I can't demonstrate a negotiation change on this specific
browser - it still falls through to bare `video/mp4`, which this
platform silently implements as VP9-in-MP4. Also found: **the resulting
file is a fragmented MP4 (`moof`/`mdat` boxes) both before and after the
fix** - per RFC-009's own contingency, this suggests the codec-string fix
alone may not be sufficient for full external-player compatibility on
every platform, though I can't confirm this on a browser with real
`avc1` support (none available here).

Question: Does Fix 2's empirical confirmation, plus Fix 1's mechanically-correct-but-unverified-benefit,
satisfy RFC-009's acceptance criteria enough to merge? Or does Fix 1's
open fragmentation question warrant scoping the remuxing fallback now
rather than waiting for Management's real-browser test?

Reason for remaining unverified items: no browser here has real H.264
support to test Fix 1's actual negotiation outcome; per RFC-009's AC4,
final external-player confirmation still needs Management's own machine
regardless of what I could verify here.

Impact if unanswered: RFC-009 stays open past its Implementation Review
Loop step 2; RENDER's output stays effectively broken for external use.
