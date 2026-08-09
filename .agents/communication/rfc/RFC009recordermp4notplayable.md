RFC-ID: RFC-009
Created: 2026-08-09
Created-By: software_architect
Target-Role: software_developer
Related-Specification: SPEC-OUTPUT-RENDER-V1 (surfaced through it, not caused by it — see Finding)
Priority: high
Status: Open

Severity: High — RENDER's entire stated purpose (produce a file you can
actually use) doesn't work: Management reports the downloaded `.mp4`
says "format isn't supported" when opened, and a second recording
attempt produced a 0-byte file.

Finding — two distinct pre-existing `Recorder` bugs, both surfaced for
the first time by RFC-008 (same reasoning as below: this code has never
actually run in the live app before now):

### Finding 1 — unplayable output (wrong container/codec)

`ui/scripts/engine/recorder.js`'s `Recorder.start()` negotiates the
`MediaRecorder` MIME type like this:

```js
let options = {};
if (MediaRecorder.isTypeSupported("video/mp4")) {
    options.mimeType = "video/mp4";
} else if (MediaRecorder.isTypeSupported("video/webm;codecs=vp9")) {
    options.mimeType = "video/webm;codecs=vp9";
} else if (MediaRecorder.isTypeSupported("video/webm")) {
    options.mimeType = "video/webm";
}
this.recorder = new MediaRecorder(stream, options);
```

**This is not new code and RFC-008 did not touch this file** (confirmed:
`git diff` of RFC-008's commit range shows zero changes to
`recorder.js`) — `Recorder`/`toggleRecord` existed already, but nothing
in the UI ever dispatched `toggleRecord` before RFC-008 added EXECUTE.
This bug was always latent; RFC-008 is simply the first time this code
has ever actually run in the live app.

The bare `"video/mp4"` check (no `codecs=` parameter) is a well-known
`MediaRecorder` pitfall: browsers can report `isTypeSupported("video/mp4")
=== true` while still producing output that either has no clearly-signaled
codec, or is a fragmented-MP4 stream (no relocated `moov`/"faststart"
layout) — the browser's own `<video>` element and the browser's own
Blob-download round-trip can play it back fine, while many other players
(QuickTime, Windows Media Player, and others that expect a conventional
MP4 layout or an explicit codec) correctly report it as unsupported. This
matches the reported symptom exactly: something downloads with a `.mp4`
extension, but external tools can't open it.

### Finding 2 — a second recording session can download an empty (0-byte) file

`ui/scripts/features/output.js`'s `toggleRecord` listener creates **one**
`Recorder` instance and reuses it across every subsequent start/stop
cycle (`if (!recorder) { recorder = new Recorder(...) }`, line 100-102).
Within `Recorder` itself (`recorder.js`), both places that touch
collected data read/write the **shared, mutable** `this.chunks`/
`this.recorder` fields directly, live, rather than each session owning
its own private, closure-captured state:

```js
// start():
this.chunks = [];
this.recorder = new MediaRecorder(stream, options);
this.recorder.ondataavailable = event => {
    if (event.data.size > 0) {
        this.chunks.push(event.data);   // reads `this.chunks` LIVE via `this`
    }
};

// stop():
this.recorder.onstop = () => {
    let blob = new Blob(this.chunks, { type: mimeType });  // reads `this.chunks` LIVE via `this`
    ...
};
try { this.recorder.stop(); } catch (error) { ... }
this.recording = false;
this.recorder = null;   // synchronous, but onstop above is async
```

`MediaRecorder.stop()` is asynchronous — the final `ondataavailable` and
then `onstop` fire *after* `stop()` returns, on their own schedule. If a
second `start()` is called before the first session's `onstop` has
actually fired (a real possibility with EXECUTE reachable back-to-back,
which is exactly what RFC-008 just made possible for the first time),
`start()`'s `this.chunks = []` reassigns the field to a brand-new array
— and because both the first session's *pending* `ondataavailable`
handler and its `onstop` handler read `this.chunks` live via `this`
rather than a value captured at their own session's start, either:
- the first session's final chunk lands in the *second* session's array
  instead of its own (silently corrupting/inflating the second file), or
- the first session's `onstop` builds its `Blob` from the *second*
  session's (still-empty-at-that-point) `this.chunks`, producing exactly
  the reported 0-byte download.

This is a real cross-session race, not a timing fluke to work around by
waiting longer between clicks — it will keep happening whenever two
recording sessions occur close enough together, which EXECUTE now makes
easy to trigger.

Required Change:

**Fix 1 (Finding 1) — negotiate an explicit, specific codec string** before falling back to
a bare container type or to webm — don't rely on the bare `"video/mp4"`
check being trustworthy. Try, in order, checking `isTypeSupported` for
each:

1. `video/mp4;codecs=avc1.42E01E` (H.264 Constrained Baseline — the most
   broadly compatible profile/level combination across players)
2. `video/mp4;codecs=avc1` (a looser H.264 request, browser picks the
   profile)
3. The existing bare `video/mp4` fallback (kept last, not first, in case
   a browser only reports the bare type as supported but still produces
   something usable)
4. `video/webm;codecs=vp9`
5. `video/webm`

Use whichever `isTypeSupported` first returns `true` from that ordered
list — same shape as the existing if/else chain, just with real codec
strings inserted ahead of the bare ones. This is a small, contained
change to `Recorder.start()`'s `options` construction only — no other
part of `Recorder` (chunk collection, `stop()`, download/Blob logic)
needs to change.

**If the codec-string fix alone doesn't produce a file real external
players accept** (possible — MediaRecorder's MP4 output can still be
fragmented/non-faststart even with an explicit codec, which is a
container-layout problem, not a codec-negotiation problem): that would
mean client-side remuxing into a standard MP4 layout is needed before
download (a materially bigger change — a small WASM/JS MP4 remuxer, or
accepting fragmented MP4 as a known limitation and documenting it rather
than promising universal player compatibility). Don't attempt that
remuxing work preemptively — try the codec-string fix first, and report
back whether that alone resolves it before scoping anything larger.

**Fix 2 (Finding 2) — give each recording session its own private
`chunks` array, not the shared `this.chunks` field.** In `start()`,
capture a local `const sessionChunks = []` and close over it directly in
`ondataavailable` (`sessionChunks.push(event.data)`, not
`this.chunks.push(...)`) instead of assigning to `this.chunks`. In
`stop()`, capture `const sessionChunks = this.chunks` (or better: have
`start()` also stash the current session's array somewhere `stop()` can
read it from, e.g. keep `this.chunks` only as "the array for whichever
session is currently in `this.recorder`", but the `onstop` closure
itself must close over that *same* local reference, not re-read
`this.chunks` at fire time) so a later session's reassignment can never
retroactively change what an in-flight `onstop`/`ondataavailable` from an
*earlier* session writes to or reads from. The concrete shape (a local
variable per session vs. some other structure) is your call — the
requirement is that no two overlapping sessions can ever read or write
each other's chunk data, by construction, not by hoping stop/start never
overlap in practice.

Acceptance Criteria:

1. `Recorder.start()`'s codec negotiation tries explicit codec strings
   before bare container types, per the ordered list above (Fix 1).
2. Each recording session's collected chunks are isolated from every
   other session's, even if a new session starts before the previous
   session's `onstop` has fired — write a test (or, if this repo's test
   harness can't exercise `MediaRecorder`/browser timing at all, a clear
   written trace of why the fix eliminates the race, per the existing
   session's constraint that no JS test harness exists here yet) proving
   two back-to-back sessions each produce their own, independently
   correct chunk set (Fix 2).
3. No other behavior of `Recorder`/`toggleRecord`/RENDER's EXECUTE
   changes — both fixes are contained to `Recorder.start()`/`stop()`.
4. **Real playback verification is required, not just "MediaRecorder
   didn't throw."** Per RFC-007's own precedent: no sandbox in this
   workflow has a browser. Record your best-effort verification (does
   `recorder.mimeType` after `start()` show the expected codec string in
   whatever environment you can check), but explicitly flag in your
   report that whether the resulting file opens in a real external
   player, and whether back-to-back recordings both produce non-empty
   files, still needs Management's confirmation on their own machine —
   don't claim this closed without that.

Acceptance Condition: Code Reviewer approval per the Implementation
Review Loop, then Management approval (real-file playback confirmation,
same pattern as RFC-007).
