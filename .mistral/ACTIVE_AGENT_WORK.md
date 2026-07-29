# ACTIVE AGENT WORK

## input-menu-fix-agent

STATUS:
IN PROGRESS

TASK:
Fix INPUT menu operations (CAMERA, VIDEO, LOAD IMAGE) to match requirements:
- None of the INPUT operations should have a [ADD] button
- A click on "LOAD IMAGE" opens the file browser
- VIDEO button should read "LOAD VIDEO" and opens file browser with video type files
- CAMERA source should use browser's Camera stream API
- None of the source operations should have a menu context

OWNERSHIP:
The following files are reserved and must not be modified by other agents:

- engine/src/operations/sources/image.rs
- engine/src/operations/sources/video.rs
- engine/src/operations/sources/camera.rs
- ui/scripts/features/image.js
- ui/scripts/features/video.js
- ui/scripts/engine/camera.js
- ui/scripts/app.js

RULES:
- Do not edit these files.
- Do not refactor surrounding code in these files.
- Do not rename public APIs.
- Do not create alternative registration systems.

DEPENDENCY CONTRACT:
When complete, this agent will provide:

- Fixed INPUT menu operations without [ADD] buttons
- LOAD IMAGE opens file browser for images
- LOAD VIDEO opens file browser for videos
- CAMERA uses browser Camera API
- No menu context for source operations

Other agents should treat the final API as the integration point.
