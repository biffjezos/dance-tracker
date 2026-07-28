import {
    getWasmApp
} from "../core/wasm.js";
import {
    getOutputNodeId,
    currentPreviewContentId
} from "./graph.js";
export function renderPreview() {
    const wasmApp = getWasmApp();
    if (!wasmApp) return;
    const canvas = document.getElementById("camera-preview");
    const id = currentPreviewContentId();
    if (id === null || id === undefined) return;
    try {
        wasmApp.preview_tick(id, canvas);
    } catch (error) {
        // expected transient failure
    }
}

function loop() {
    const wasmApp = getWasmApp();
    const outputNodeId = getOutputNodeId();
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