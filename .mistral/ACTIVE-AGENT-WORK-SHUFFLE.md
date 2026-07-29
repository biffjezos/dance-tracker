# ACTIVE AGENT WORK

## shuffle-agent

STATUS:
IN PROGRESS

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

1. [ ] Fix parameter name mismatch between Rust and JavaScript
2. [ ] Verify the fix compiles
3. [ ] Test the operation works correctly
4. [ ] Commit and push changes to dev branch
