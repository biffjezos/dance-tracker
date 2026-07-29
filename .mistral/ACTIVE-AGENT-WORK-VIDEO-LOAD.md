# ACTIVE AGENT WORK

## video-load-agent

STATUS:
IN PROGRESS

TASK:
Implement VideoSource loading for MP4 files into the app, similar to Image loading. Fix bugs, add missing WASM bindings, UI, and infrastructure.

OWNERSHIP:
The following files are reserved and must not be modified by other agents:

- ui/scripts/features/video.js
- engine/src/operations/sources/video.rs
- engine/src/graphics/video.rs
- engine/src/compositor/value.rs
- engine/src/renderer/mod.rs
- engine/src/app.rs
- engine/src/dom.rs

RULES:
- Do not edit these files.
- Do not refactor surrounding code in these files.
- Do not rename public APIs.
- Do not create alternative video loading systems.

DEPENDENCY CONTRACT:
When complete, this agent will provide:

- Fixed video.js with no duplicate content
- VideoSource that supports HTMLVideoElement via PixelSource
- Value::Video enum variant
- ToRenderFrame implementation for Video
- Working WASM bindings for video element
- Video loading functionality similar to Image loading

Other agents should treat the final API as the integration point.

## Sub-tasks:
1. Fix video.js - remove duplicate content
2. Add Value::Video to Value enum
3. Update VideoSource to support PixelSource
4. Add ToRenderFrame for Video
5. Fix WASM bindings in app.rs
6. Test and verify video loading works
