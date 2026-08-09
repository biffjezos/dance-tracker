# Evaluation: RFC-009 (high) — Recorder mp4/race fix (RFI-RFC-009-READY)

**Branch:** `claude/dev-session-plw4fq` (not yet merged), commit `ad0bc76`
**RFC:** `RFC009recordermp4notplayable.md` (Software Architect, high — surfaced via RFC-008)
**Report:** `.agents/communication/implementation_reports/RFC009_recorder_mp4_fix_report.md`
**File touched:** `ui/scripts/engine/recorder.js` only. Confirmed via `git diff origin/dev..HEAD --stat`.

## 0. This review includes independent empirical reproduction, not just a code trace

This sandbox also has working headless Chromium (`/opt/pw-browsers`) and Playwright is
globally available (`NODE_PATH=/opt/node22/lib/node_modules`). Rather than accept the
report's own before/after numbers, I built a separate harness from scratch — different
code, same real `recorder.js` module, same technique (drive `Recorder` against a real
`<canvas>` in a real browser, intercept `URL.createObjectURL` to capture the downloaded
`Blob`s, parse MP4 box headers by hand) — and reran the comparison myself.

## 1. Code trace — the fix, read directly

`start()`: `const sessionChunks = []; this.chunks = sessionChunks;` — `ondataavailable`'s
closure pushes into `sessionChunks` directly (line 56), never re-reading `this.chunks`.
`stop()`: `const sessionChunks = this.chunks;` captured synchronously (line 79), **before**
`this.recorder.stop()` is called (line 100) — since JS is single-threaded and there is no
`await`/yield between that capture and the synchronous code that follows, no subsequent
`start()` call can interleave and reassign `this.chunks` before this session's `onstop`
closure (which closes over this same `sessionChunks` reference, not `this.chunks`) is
fully wired up. The old `MediaRecorder` object instance from a prior session keeps firing
its own bound `ondataavailable`/`onstop` independently of `this.recorder` being reassigned
to a new instance in a later `start()` — reassigning a property never affects an
already-created object's own event delivery. This is structurally correct: no two
overlapping sessions can share state, by construction, not by hoping timing works out. ✅
(AC2, AC3)

## 2. Independent empirical reproduction — original code

Own harness, zero-gap back-to-back sessions (`stop()` immediately followed by `start()`),
against `recorder.js` as it exists on `origin/dev` (pre-fix):

```
session 0: 2923 bytes, ftyp occurrences: 1
session 1: 5722 bytes, ftyp occurrences: 2   <- concatenated (~2x the size, 2 ftyp boxes)
```

Reproduces the exact corruption RFC-009 Finding 2 and the report describe — two complete
MP4 streams concatenated into session 1's download. Confirms this is a real, reproducible
defect independent of the developer's own harness/numbers.

## 3. Independent empirical reproduction — fixed code

Identical harness and timing, against the fix (`ad0bc76`), 5 runs:

```
run 1: session 0: 2841 bytes (1 ftyp), session 1: 2877 bytes (1 ftyp)
run 2: session 0: 2841 bytes (1 ftyp), session 1: 2796 bytes (1 ftyp)
run 3: session 0: 2840 bytes (1 ftyp), session 1: 2886 bytes (1 ftyp)
run 4: session 0: 2831 bytes (1 ftyp), session 1: 2737 bytes (1 ftyp)
run 5: session 0: 2831 bytes (1 ftyp), session 1: 2888 bytes (1 ftyp)
```

5/5 clean — every session, both slots, exactly one `ftyp` box, no concatenation, sizes
consistent run-to-run. **Fix 2 independently confirmed eliminated, empirically, not just
by code trace.** (AC2)

## 4. Fix 1 — codec negotiation, independently checked on this sandbox too

```js
MediaRecorder.isTypeSupported('video/mp4;codecs=avc1.42E01E') -> false
MediaRecorder.isTypeSupported('video/mp4;codecs=avc1')         -> false
MediaRecorder.isTypeSupported('video/mp4')                     -> true
MediaRecorder.isTypeSupported('video/webm;codecs=vp9')          -> true
```

Matches the report's own finding exactly: this sandbox's Chromium also has no H.264
encoder, so Fix 1's ordering change is mechanically present and correctly ordered (AC1)
but doesn't change the negotiated outcome *on this platform* — falls through to the same
bare `video/mp4` branch either way. Not a defect in the fix; a real, independently-
confirmed platform limitation. (AC1 — code correct; real-world benefit unconfirmable here)

## 5. Fragmentation — independently confirmed via box-structure parse

Parsed session 0's box structure by hand (own Python, not reusing the report's method):

```
[('ftyp', 36), ('moov', 695), ('moof', 240), ('mdat', 1784), ('mfra', 76)]
```

`moof`/`mdat`/`mfra` present alongside `moov` — a fragmented MP4 layout, both before and
after the fix, confirming the report's claim independently. This is a container-layout
property Fix 1 doesn't address (it only changes codec negotiation, not the fragmentation
the RFC itself flagged as a possibly-separate follow-up concern).

## 6. Scope

Diff is exactly `recorder.js` — confirmed via `git diff --stat`. No other file (`output.js`,
`toggleRecord`'s listener, RENDER's operation code) touched, matching RFC-009's constraint
that both fixes stay contained to `Recorder.start()`/`stop()`. ✅ (AC3)

## 7. What remains genuinely unverified — same as the report's own honest accounting

Neither this session nor the developer's has a browser with real H.264/`avc1` support, so
whether Fix 1 actually produces a non-fragmented, universally-playable file on a platform
that *does* negotiate `avc1` is unconfirmable here — this is a platform gap in every
sandbox available to this workflow, not a gap in the fix or in either review. Per RFC-009's
own AC4, real external-player confirmation (QuickTime, Windows Media Player, or similar)
still requires Management's own machine regardless of this approval.

## Decision

**Approve.** Fix 2 (the cross-session race, the more severe of the two defects — corrupted/
oversized downloads) is now empirically confirmed eliminated by two independent harnesses
(the developer's and mine), not just a shared source-level claim. Fix 1 is mechanically
correct and correctly ordered per the RFC's required list; its real-world benefit is
unconfirmable in any sandbox available to this workflow (no `avc1` support anywhere), which
is an environment ceiling, not a defect. Scope is exactly `recorder.js`, nothing else
touched. Per RFC-009's own AC4, forward to Management for final real-player confirmation —
same pattern as RFC-007 — this approval covers the Code Review step of the Implementation
Review Loop. On the open question the developer raised (scope the remuxing fallback now,
or wait for Management's real-browser test): **wait.** Neither of us can confirm fragmentation
actually blocks a real external player once `avc1` is negotiated for real — building a
remuxer against an unconfirmed problem risks solving the wrong thing; Management's
real-browser result should decide whether that work is even needed.
