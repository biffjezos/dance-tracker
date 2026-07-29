// app.js
import { getOperations, executeOperation } from "./core/operations.js";
import { initWasm } from "./core/wasm.js";

import { WIDTH, HEIGHT } from "./engine/constants.js";
import { Camera } from "./engine/camera.js";
import { MenuManager } from "./engine/menu.js";
import { startRenderLoop } from "./engine/render.js";
import { applyOutputSize } from "./features/output.js";
import { reportSelection } from "./engine/status.js";
import { nodeSelectionState } from "./engine/nodeSelection.js";

// Import image feature to register event listeners
import "./features/image.js";

const settings = {
    video: {
        width: WIDTH,
        height: HEIGHT
    }
};

const camera = new Camera(settings);

const menu = new MenuManager();
menu.init();
window.addEventListener("menuOperation", e => { executeOperation(e.detail); });
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
        new CustomEvent("operationsLoaded", {
            detail: operations
        })
    );

    startRenderLoop();
}

boot();
