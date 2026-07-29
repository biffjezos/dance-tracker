# ACTIVE AGENT WORK

## video-source-agent

STATUS:
DONE

TASK:
Implement VideoSource operation based on ImageSource implementation.

OWNERSHIP:
The following files are reserved and must not be modified by other agents:

- engine/src/operations/sources/video.rs
- engine/src/operations/sources/mod.rs
- engine/src/app.rs
- ui/scripts/features/video.js

RULES:
- Do not edit other source files unless necessary for VideoSource to work
- Follow the same pattern as ImageSource
- Use appropriate naming (video_source, LOAD VIDEO, Video Source)
- Keep the same descriptor structure but with video-specific labels

DEPENDENCY CONTRACT:
When complete, this agent will provide:

- A working VideoSource operation that mirrors ImageSource functionality
- Proper registration in the inventory system
- Appropriate metadata and descriptors
- WASM bindings for creating video source nodes and setting video data
- UI handler for loading video files and extracting frames

Other agents should treat the VideoSource as a source operation similar to ImageSource.

COMPLETED:
- Implemented VideoSource struct with image storage (Option<Arc<Image>>)
- Added set_image and get_image methods to VideoSource
- Implemented Operation trait for VideoSource with proper descriptor and metadata
- Exported VideoSource from sources/mod.rs
- Added VideoSource import to app.rs
- Added create_video_source_node method to App
- Added set_video_on_node method to App
- Updated video.js to use WASM VideoSource node
- Added extractVideoFrame helper function
- Video files are now loaded, first frame extracted, and set on VideoSource node
- All changes committed and pushed to dev branch

---

## preview-canvas-agent

STATUS:
DONE

TASK:
Fix preview canvas not updating when NODE SELECTOR under menu NODES selects a different Node. The preview canvas must display the selected node's output, and the TITLE of the canvas must change to the NAME of the active NODE. If the node is an operation that has got no visuals, display nothing in the canvas.

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

COMPLETED:
- Added nodeSelectionChanged event dispatch in nodeSelection.js setSelectedNode()
- Added event listener in status.js to update camera panel title on node selection change
- Updated render.js to clear preview canvas when node has no visuals or preview fails
- All changes committed and pushed to dev branch
