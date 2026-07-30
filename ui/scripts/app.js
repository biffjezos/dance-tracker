import { getOperations, executeOperation } from "./core/operations.js";
import { initWasm, getWasmApp } from "./core/wasm.js";

import { WIDTH, HEIGHT } from "./engine/constants.js";
import { Camera } from "./engine/camera.js";
import { MenuManager } from "./engine/menu.js";
import { startRenderLoop } from "./engine/render.js";
import { applyOutputSize } from "./features/output.js";
import { reportSelection } from "./engine/status.js";

import { nodeSelectionState } from "./engine/nodeSelection.js";
import "./engine/nodeEditContexts.js";
import { initCanvasFocus, getFocusedPanel } from "./engine/canvasFocus.js";
import { togglePlayback } from "./engine/transport.js";
import { initPanelExpand } from "./engine/panelExpand.js";

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

// Space play/stops the selected node's video, but only when a canvas is
// focused (clicked) - otherwise Space scrolling the page or doing nothing
// unexpectedly would be surprising.
window.addEventListener("keydown", e => {
    if (e.code !== "Space" || !getFocusedPanel()) return;

    const selectedNode = nodeSelectionState.getSelectedNode();
    const videoEl = selectedNode?.layer?.videoEl;
    if (!videoEl) return;

    e.preventDefault();
    togglePlayback(videoEl);
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