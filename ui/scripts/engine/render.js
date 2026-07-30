// engine/render.js
import {
    getWasmApp
} from "../core/wasm.js";
import {
    getOutputNodeId,
    currentPreviewContentId
} from "./graph.js";

let liveNodeId = null;
export function renderPreview() {
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

window.addEventListener("setLiveNode", e => {
    liveNodeId = e.detail;
});
window.addEventListener("clearLiveNode", () => {
    liveNodeId = null;
});

function loop() {
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
    renderPreview();
    requestAnimationFrame(loop);
}
export function startRenderLoop() {
    requestAnimationFrame(loop);
}
