# ACTIVE AGENT WORK

## shuffle-agent

STATUS:
COMPLETE

TASK:
Fix and complete the SHUFFLE operation implementation. The shuffle operation exists but has a parameter name mismatch between Rust and JavaScript UI code.

OWNERSHIP:
The following files are reserved and must not be modified by other agents:

- engine/src/operations/transform/shuffle.rs
- engine/src/app.rs (shuffle-related parts)
- ui/scripts/engine/nodeEditContexts.js (shuffle parts)
- ui/scripts/engine/menu.js (shuffle parts)
- ui/scripts/engine/state.js (shuffle parts)

RULES:
- Do not edit these files for unrelated purposes.
- Do not refactor surrounding code in these files.
- Do not rename public APIs.
- Do not create alternative shuffle implementations.

DEPENDENCY CONTRACT:
When complete, this agent will provide:

- Fixed parameter name consistency between Rust and JavaScript
- Working SHUFFLE operation that correctly remaps RGBA channels
- Proper UI integration for channel selection

Other agents should treat the final API as the integration point.

## Tasks Breakdown

1. [x] Fix parameter name mismatch between Rust and JavaScript
2. [x] Verify the fix compiles
3. [x] Test the operation works correctly
4. [x] Commit and push changes to dev branch

## Summary of Changes

Fixed the parameter names in `ui/scripts/engine/nodeEditContexts.js` to match the Rust implementation:
- Changed `channels` array from `["red", "green", "blue", "alpha"]` to `["red_channel", "green_channel", "blue_channel", "alpha_channel"]`

This ensures that when users click the channel buttons in the UI, the parameter updates are correctly routed to the Rust Shuffle operation.

The SHUFFLE operation now correctly:
- Remaps RGBA channels based on user selection
- Supports setting each output channel (R, G, B, A) to any input channel (R, G, B, A, OFF)
- Works with the existing UI controls
- Is properly registered in the inventory system
