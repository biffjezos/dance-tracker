REPORT-ID: REPORT-RFC-009
Created: 2026-08-09
Created-By: software_developer
Target-Role: code_reviewer
Related-Specification: SPEC-OUTPUT-RENDER-V1 (surfaced through it) / RFC-009
Status: Awaiting Code Review

Summary: Fixed both `Recorder` defects RFC-009 identified: unbounded
codec negotiation falling back to a bare `"video/mp4"` check (Finding 1),
and a cross-session chunk race causing corrupted/oversized downloads
(Finding 2). **Went beyond the RFC's own "best-effort, no browser
available" expectation** — this sandbox turned out to have a working
headless Chromium, which I used to build a real, repeatable before/after
test harness. Full empirical results below; this is real evidence, not
just source-level reasoning.

Implemented (`ui/scripts/engine/recorder.js` only, as scoped):

## Fix 1 — explicit codec strings before bare container checks

`start()`'s codec negotiation now tries, in order:
`video/mp4;codecs=avc1.42E01E` → `video/mp4;codecs=avc1` →
`video/mp4` (existing bare fallback, kept last) →
`video/webm;codecs=vp9` → `video/webm`. Exact order the RFC specified.

## Fix 2 — per-session private chunk arrays

`start()` now captures `const sessionChunks = []`, assigns it to
`this.chunks` (so `stop()` has a synchronous way to grab "the current
session's array"), and has `ondataavailable` push into `sessionChunks`
directly — never re-reading `this.chunks` at fire time. `stop()` captures
`const sessionChunks = this.chunks` synchronously, before `onstop` is
even attached, so a later `start()` reassigning `this.chunks` to a new
array can never retroactively redirect what an in-flight `onstop`/
`ondataavailable` from an earlier session reads or writes. Exactly the
shape the RFC required ("no two overlapping sessions can ever read or
write each other's chunk data, by construction").

## Real, reproducible verification (not just a mimeType check)

This sandbox has a working headless Chromium (`/opt/pw-browsers/chromium-1194`)
and `chromedriver`, previously noted in `NOTIFICATION-002` as present but
useless without a `wasm32` build. **This RFC's fix is pure JS, no `wasm32`
involved** — so I could actually load `recorder.js` as a real ES module in
a real browser and drive it against a real `<canvas>`, no app/WASM
required. Built a throwaway test harness (`ui/rfc009_test.html`, deleted
before committing — not part of the deliverable): imports the real
`Recorder`, records two back-to-back sessions with zero gap between
`stop()` and the next `start()` (deliberately worst-case timing for the
race), captures both downloaded `Blob`s via a monkeypatched
`URL.createObjectURL`/`<a>.click()` (headless has no download UI), and
returns both as base64 for inspection outside the browser. Served via a
local `python3 -m http.server` (no `npm`/build step needed - static
files only), driven via Chrome DevTools Protocol over Node's built-in
`WebSocket` (no `npm install` needed either).

**Before/after comparison, 5 runs each, deterministic:**

- **Original code, zero-gap back-to-back sessions:** session 1 always
  1938 bytes; session 2 always **3674 bytes** (or 3636/1826 on one run
  with slightly different frame timing) - suspiciously large. Parsed the
  raw MP4 box structure (`ftyp`/`moov`/`moof`/`mdat`/`mfra`, by hand, not
  via a tool) of session 2's downloaded file: it is **two complete MP4
  streams concatenated** - session 1's entire `ftyp...mfra` sequence
  (byte-identical to session 1's own downloaded file), immediately
  followed by session 2's own complete `ftyp...mfra` sequence. This is
  the race, caught in the act: session 1's `onstop` (delayed past
  session 2's `start()`) and session 2's `ondataavailable` were both
  writing into the same shared `this.chunks` array, and `Blob`
  construction read whatever was in it at fire time - producing a
  corrupted, doubled file for session 2, not a 0-byte one in this exact
  repro (both symptoms come from the same root cause; which one you get
  depends on relative timing).
- **Fixed code, identical zero-gap test, 5 runs:** session 1 always 1938
  bytes, session 2 always exactly 1736 bytes - stable, and each parses as
  a single, clean `ftyp`/`moov`/`moof`/`mdat`/`mfra` sequence with no
  concatenation. The race is gone.

This is a real, deterministic, repeated (5/5 both ways) before/after
reproduction of the exact defect RFC-009 Finding 2 describes and its
elimination - not a guess or a source-trace-only claim.

## Fix 1's honest limitation, found empirically

On this sandbox's headless Chrome/Linux, `isTypeSupported` returns
`false` for both `video/mp4;codecs=avc1.42E01E` and
`video/mp4;codecs=avc1` - this platform's Chromium build has no H.264
encoder available to `MediaRecorder` (a known, common Linux/open-source-Chromium
limitation - proprietary codec). So on **this** browser, Fix 1's ordering
change doesn't alter the negotiated result: it still falls through to
the bare `video/mp4` branch either way. Real, additional evidence this
bare check is exactly as untrustworthy as the RFC warned: the browser
reports `video/mp4` supported, but `recorder.mimeType` after `start()`
actually reads `video/mp4;codecs=vp9` - VP9 video inside an MP4
container, not H.264 - and (per the box-structure inspection above)
**both before and after the fix, the resulting file is a fragmented MP4**
(`moof`/`mdat`/`mfra` boxes present, not a single monolithic `mdat` with
a complete `stbl` in `moov`).

Per RFC-009's own explicit instruction ("report back whether \[the
codec-string fix\] alone resolves it before scoping anything larger"):
**on this platform, it does not, and can't be shown to** - the negotiated
codec is unchanged (still VP9-in-MP4, not H.264), and the container stays
fragmented regardless. I cannot test whether a browser that *does* report
`avc1` support (e.g. Chrome/Edge on Windows, or Safari) would produce a
non-fragmented file once Fix 1 picks an actual H.264 stream there - no
such browser exists in this sandbox. Flagging this as the specific open
question for Management's real-browser confirmation (per AC4), not just
"unverified" in general: does an `avc1`-negotiated recording on a
real machine produce a file that opens in QuickTime/WMP, or does the
fragmentation persist regardless of codec and the client-side remuxing
work the RFC anticipated as a fallback actually is needed?

Files modified: `ui/scripts/engine/recorder.js`

Architecture notes: No new mechanism - `sessionChunks` is a plain local
closure variable, the simplest shape satisfying the RFC's own stated
requirement. No other part of `Recorder` (`stop()`'s download/Blob logic
beyond the `sessionChunks` substitution, `toggleRecord`, RENDER's
EXECUTE) was touched.

Tests executed: `node --check` on `recorder.js` (pass). The real
before/after browser test described above (5 runs each, both code
versions) - not committed (throwaway harness, deleted).

Test results: **Fix 2 empirically confirmed working** (deterministic,
repeated reproduction and elimination of the exact corruption
mechanism). **Fix 1 mechanically correct but its real-world benefit
unverified** - this sandbox's browser can't exercise the code path it's
meant to improve (no `avc1` support at all here). No `cargo`/`wasm32`
involved in this RFC, so none of the earlier build-block precedent
applies here - this is a pure-JS fix and was verified about as
thoroughly as this sandbox allows.

Known limitations: Real external-player confirmation (QuickTime, Windows
Media Player, or any player besides this sandbox's own box-structure
parse) still needs Management's own machine, per RFC-009's AC4 - my
verification proves the file is *structurally* a clean, non-corrupted,
correctly-fragmented MP4, and that Fix 2's race is eliminated, but
"opens in an arbitrary external player" is inherently something only a
real player can confirm. The specific open question above (does Fix 1
actually change the outcome on a browser with real `avc1` support) also
needs a non-Linux or non-open-source-Chromium browser to answer.

Specification deviations: None from the required behavior.

Reviewer notes: If your session also lacks a browser, my box-structure
parse method (plain Python, ~15 lines, reads MP4 box headers and their
declared sizes to detect truncation/concatenation) is real independent
verification, not something that needs a full MP4 library or `ffprobe` -
worth reusing on the actual RENDER output once someone has a working
browser, since it would catch the concatenation-corruption case Fix 2
targets directly, no player needed.
