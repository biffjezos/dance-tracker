// engine/render.js
import {
    getWasmApp
} from "../core/wasm.js";
import {
    getOutputNodeId,
    currentPreviewContentId
} from "./graph.js";
import { isPanelVisible } from "./panelExpand.js";

let liveNodeId = null;
let liveNodeLabel = null;
const LIVE_OUTPUT_DEFAULT_TITLE = "LIVE OUTPUT";

export function renderPreview() {
    if (!isPanelVisible("preview")) return;

    const wasmApp = getWasmApp();
    if (!wasmApp) return;
    const canvas = document.getElementById("camera-preview");
    const id = currentPreviewContentId();
    if (id === null || id === undefined) {
        // Clear the canvas if there's no valid preview ID
        const ctx = canvas.getContext("2d");
        if (ctx) {
            ctx.clearRect(0, 0, canvas.width, canvas.height);
        }
        return;
    }
    try {
        wasmApp.preview_tick(id, canvas);
    } catch (error) {
        // If preview fails (e.g., node has no visuals), clear the canvas
        const ctx = canvas.getContext("2d");
        if (ctx) {
            ctx.clearRect(0, 0, canvas.width, canvas.height);
        }
    }
}

// What node the output panel is actually showing right now - a live-preview
// override if one is set, otherwise the wired LIVE OUTPUT node. Used by the
// Space-bar transport to find the video(s) actually driving that panel.
export function getDisplayedOutputNodeId() {
    return liveNodeId ?? getOutputNodeId();
}

window.addEventListener("setLiveNode", e => {
    liveNodeId = e.detail.nodeId;
    liveNodeLabel = e.detail.label;
});
window.addEventListener("clearLiveNode", () => {
    liveNodeId = null;
    liveNodeLabel = null;
});

function loop() {
    const liveOutputTitle = document.getElementById("live-output-title");
    if (liveOutputTitle) {
        liveOutputTitle.innerText = liveNodeId !== null ? liveNodeLabel : LIVE_OUTPUT_DEFAULT_TITLE;
    }

    if (isPanelVisible("output")) {
        const wasmApp = getWasmApp();
        const outputNodeId = liveNodeId ?? getOutputNodeId();
        const masterCanvas = document.getElementById("master-layer");

        if (wasmApp && outputNodeId !== null && outputNodeId !== undefined) {
            try {
                wasmApp.render_tick(outputNodeId, masterCanvas);
            } catch (error) {
                // expected transient failure
            }
        } else {
            masterCanvas.getContext("2d").clearRect(0, 0, masterCanvas.width, masterCanvas.height);
        }
    }
    renderPreview();
    requestAnimationFrame(loop);
}
export function startRenderLoop() {
    requestAnimationFrame(loop);
}
