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