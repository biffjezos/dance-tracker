# ACTIVE AGENT WORK

## input-menu-fix-agent

STATUS:
DONE

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

COMPLETED:
- Removed create_node from all three source operations (image, video, camera)
- Changed VIDEO label to "LOAD VIDEO"
- Changed camera ui_action to "open_camera_stream"
- Added open_video_picker handler in video.js
- Added open_camera_stream handler in video.js
- All changes committed and pushed to dev branch

---

## preview-canvas-agent

STATUS:
IN PROGRESS

TASK:
Fix preview canvas not updating when NODE SELECTOR under menu NODES selects a different Node. The preview canvas must display the selected node's output, and the TITLE of the canvas must change to the NAME of the active NODE. If the node has no visuals (e.g., unconnected shuffle operation), display nothing in the canvas.

OWNERSHIP:
The following files are reserved and must not be modified by other agents:

- ui/scripts/engine/nodeSelection.js
- ui/scripts/engine/menu.js
- ui/scripts/engine/render.js
- ui/scripts/engine/graph.js
- ui/scripts/engine/status.js
- ui/index.html

RULES:
- Do not edit these files.
- Do not refactor surrounding code in these files.
- Do not rename public APIs.
- Do not create alternative node selection systems.

DEPENDENCY CONTRACT:
When complete, this agent will provide:

- Preview canvas updates when node selection changes
- Camera panel title updates to show selected node name
- Proper handling of nodes with no visual output

Other agents should treat the final API as the integration point.
