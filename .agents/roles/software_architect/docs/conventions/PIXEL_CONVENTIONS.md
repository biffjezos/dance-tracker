<!-- .agents/roles/software_architect/docs/conventions/PIXEL_CONVENTIONS.md -->

# Pixel conventions (colorspace & alpha)

This file records two core decisions about how pixel data is represented
and blended across the engine - made once, deliberately, so they don't
need re-deciding (and every existing/future operation re-touched) each
time a new one is added. See `CLAUDE.md` for how this file relates to the
rest of the project's standing rules.

## Colorspace: untagged, by design, until something needs otherwise

**Decision:** every pixel-bearing type (`U8Image`, `FloatImage`, `Frame`)
carries a `colorspace: ColorSpace` field. Today `ColorSpace` has exactly
one variant, `Untagged` - meaning "whatever encoding the source pixels
arrived in, unconverted, uninterpreted." No operation in the graph
performs any colorspace conversion; RGB channel values are passed through
and math'd on as raw numbers regardless of whether the original source
was sRGB-encoded video, a camera stream, or a generated pattern.

**Why decide this now instead of leaving it unstated:** without an
explicit field, "no colorspace management" is an *assumption* every
operation's author has to independently know and get right forever. With
the field (even at one variant), it's a *fact* the type carries - a
future operation that legitimately needs to know (linear-light blur, a
colorspace-aware key) can check it and fail loudly on `Untagged` instead
of silently producing wrong output. Adding the field now, while a small
number of operations exist, costs one field default per constructor;
deferring it costs the same edit spread across however many operations
exist by the time someone actually needs it.

**What this means for new operations:**
- Every place that constructs a `U8Image`/`FloatImage`/`Frame` sets
  `colorspace: ColorSpace::Untagged` explicitly - never omit it, never
  guess a "real" value (`Srgb`, `Linear`) that hasn't actually been
  verified against how the source was decoded.
- If/when real color management is implemented (linear-light math for
  resize/blur, sRGB<->linear conversion around a key), it lands as new
  `ColorSpace` variants plus explicit conversion operations - never a
  silent default change to existing operations' behavior.
- This is a documentation/discipline convention, not (yet) a type-level
  guarantee - nothing currently rejects mixing colorspaces at runtime.

**Current state:** the field does not exist in the code yet - this is the
decision, not the patch. Implementing it means adding a
`graphics::color::ColorSpace` enum (one variant, `Untagged`, to start),
threading a `colorspace` field onto `U8Image`/`FloatImage`/`Frame`, and
setting it explicitly at every construction site (every operation that
builds one of these three types, plus `graphics::mask`'s pixel helpers).

## Alpha: straight, uniform-channel, by design - not "over" compositing

**Decision:** alpha is always straight (never premultiplied) everywhere
in the engine. Separately: `Add`/`Multiply`/`Screen` (and any future
same-shape COMPOSE operation) blend all 4 channels - R, G, B, *and* A -
identically and uniformly. This is a deliberate stylistic choice for a
demo-scene visual synthesizer, where alpha is just another synthesizable
channel like color, not a coverage mask to be treated specially. **It is
not a standard front-to-back "over" operator**, and should not be
mistaken for one or "fixed" to behave like one.

**Why decide this now instead of leaving it implicit:** three operations
(`add.rs`, `multiply.rs`, `screen.rs`) already independently encode "treat
all 4 channels the same" as their blend math, and their own tests lock
that behavior in. Without writing down that this is intentional, a future
session (or an audit) is likely to flag it as a bug - alpha usually *is*
special in compositing - and "fix" it in a way that breaks existing,
deliberate behavior and its test coverage.

**What this means for new operations:**
- New COMPOSE-menu math operations (e.g. a future Divide, or another
  Subtract-shaped op) default to the same uniform 4-channel treatment as
  Add/Multiply/Screen, for consistency with what's already shipped.
- The one place alpha *is* used semantically today is
  `graphics::mask::apply_mask` - a wired MASK input's alpha channel is
  read as a 0..1 blend weight between an operation's original and
  processed output. That's a deliberate, narrow, already-correct
  exception, not a precedent for compositing operations to start reading
  foreground alpha as coverage too.
- A real front-to-back "over"/"merge" operator (alpha-weighted
  `fg * a + bg * (1 - a)` on RGB, standard alpha-out math) is a **new,
  separate operation** if it's ever wanted - explicitly documented as
  breaking from this convention - never a rewrite of Add/Multiply/
  Screen's existing semantics.

**Current state:** already implemented and already the codebase's real
behavior (see `operations/compose/{add,multiply,screen}.rs` and their
tests, `graphics/mask.rs`'s `apply_mask`) - this section exists to record
that it's intentional, not to describe a pending change.
