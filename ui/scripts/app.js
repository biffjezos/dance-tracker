import { getOperations, executeOperation } from "./core/operations.js";
import { initWasm, getWasmApp } from "./core/wasm.js";

import { WIDTH, HEIGHT } from "./engine/constants.js";
import { Camera } from "./engine/camera.js";
import { MenuManager } from "./engine/menu.js";
import { startRenderLoop } from "./engine/render.js";
import { applyOutputSize } from "./features/output.js";
import { reportSelection } from "./engine/status.js";

import "./engine/nodeEditContexts.js";
import { initCanvasFocus, getFocusedPanel } from "./engine/canvasFocus.js";
import { initPanelExpand } from "./engine/panelExpand.js";
import { currentPreviewContentId, findVideoElementsForNode } from "./engine/graph.js";
import { getDisplayedOutputNodeId } from "./engine/render.js";

import "./features/image.js";
import "./features/video.js";


const settings = {
    video: {
        width: WIDTH,
        height: HEIGHT
    }
};


const camera = new Camera(settings);

const menu = new MenuManager();
menu.init();

initCanvasFocus();
initPanelExpand();

// Space play/stops the video(s) actually driving whatever the focused
// canvas is showing right now - not just a raw video/camera layer that
// happens to be selected, but walked upstream through the graph, since the
// displayed node is often something downstream (a blend mode, a transform)
// rather than the source itself. Only fires when a canvas is focused
// (clicked) - otherwise Space scrolling the page unexpectedly would be
// surprising.
//
// TODO(independent per-panel playback): this toggles the shared
// videoEl.play()/.pause() directly, so the same CAMERA/VIDEO node can't be
// playing for one panel's chain and paused for another's - there's only
// one clock. Planned fix: move to a per-executor freeze-hold instead (see
// Operation::is_live() in the Rust engine, already added) - each panel
// gets its own frozen/live map keyed by node id, the underlying DOM
// element always keeps playing, and Space toggles that panel's freeze
// state via a new WASM binding rather than touching videoEl directly.
window.addEventListener("keydown", e => {
    // A canvas click sets focusedPanel and stays set until another canvas
    // is clicked - it doesn't track which element currently has keyboard
    // focus. Without this check, typing a space into any text field (e.g.
    // renaming a node) while a panel was previously clicked would both
    // fail to type the space and toggle playback at the same time.
    const target = e.target;
    if (target && (target.tagName === "INPUT" || target.tagName === "TEXTAREA")) return;

    const panel = getFocusedPanel();
    if (e.code !== "Space" || !panel) return;

    const nodeId = panel === "output"
        ? getDisplayedOutputNodeId()
        : currentPreviewContentId();

    const videoEls = findVideoElementsForNode(nodeId);
    if (videoEls.length === 0) return;

    e.preventDefault();

    // There's no single canonical video once more than one feeds the same
    // node - toggle them all the same way, based on the first one's state,
    // so they move together rather than fighting each other's paused state.
    const shouldPlay = videoEls[0].paused;
    videoEls.forEach(videoEl => {
        if (shouldPlay) {
            videoEl.play().catch(() => {});
        } else {
            videoEl.pause();
        }
    });
});


window.addEventListener("menuOperation", e => {
    executeOperation(e.detail);
});


window.addEventListener("updateNodeParameter", e => {

    const {
        nodeId,
        parameter,
        value
    } = e.detail;

    console.log(
        "Update node parameter:",
        nodeId,
        parameter,
        value
    );

    const wasmApp = getWasmApp();

    if (!wasmApp) {
        return;
    }

    wasmApp.update_node_parameter(
        nodeId,
        parameter,
        value
    );
});


document.getElementById("master-layer").width = WIDTH;
document.getElementById("master-layer").height = HEIGHT;


document.getElementById("camera-preview").width = WIDTH;
document.getElementById("camera-preview").height = HEIGHT;



async function boot() {

    await initWasm();

    applyOutputSize();

    reportSelection("video");


    const operations = getOperations();


    window.dispatchEvent(
        new CustomEvent(
            "operationsLoaded",
            {
                detail: operations
            }
        )
    );


    startRenderLoop();
}


boot();