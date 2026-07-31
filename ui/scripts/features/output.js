// features/output.js

import {
    WIDTH,
    HEIGHT
} from "../engine/constants.js";
import {
    state
} from "../engine/state.js";
import {
    getWasmApp
} from "../core/wasm.js";
import {
    Recorder
} from "../engine/recorder.js";
/*
==================================================
OUTPUT / RECORDING
==================================================
*/
const OUTPUT_RESOLUTIONS = [{
        width: 320,
        height: 240
    }, // QVGA, 4:3
    {
        width: 640,
        height: 480
    }, // VGA, 4:3
    {
        width: 800,
        height: 600
    }, // SVGA, 4:3
    {
        width: 1024,
        height: 768
    }, // XGA, 4:3
    {
        width: 1280,
        height: 720
    }, // HD, 16:9
    {
        width: 1024,
        height: 1024
    }, // square
    {
        width: 1600,
        height: 1200
    }, // UXGA, 4:3
    {
        width: 1920,
        height: 1080
    }, // Full HD, 16:9
    {
        width: 2560,
        height: 1440
    }, // QHD, 16:9
    {
        width: 3840,
        height: 2160
    } // 4K UHD
];
state.outputResolutionIndex = OUTPUT_RESOLUTIONS.findIndex(r => r.width === WIDTH && r.height === HEIGHT);
state.outputWidth = WIDTH;
state.outputHeight = HEIGHT;
export function applyOutputSize() {
    const wasmApp = getWasmApp();
    if (!wasmApp) return;
    wasmApp.set_resolution(state.outputWidth, state.outputHeight);
    const aspectRatio = (state.outputWidth / state.outputHeight).toFixed(3);
    const bar = document.querySelector(".statusbar");
    if (bar && bar.children[4]) {
        bar.children[4].innerText = state.outputWidth + "x" + state.outputHeight + " " + aspectRatio;
    }
}
window.addEventListener("outputSizeUp", () => {
    state.outputResolutionIndex = Math.min(OUTPUT_RESOLUTIONS.length - 1, state.outputResolutionIndex + 1);
    state.outputWidth = OUTPUT_RESOLUTIONS[state.outputResolutionIndex].width;
    state.outputHeight = OUTPUT_RESOLUTIONS[state.outputResolutionIndex].height;
    applyOutputSize();
});
window.addEventListener("outputSizeDown", () => {
    state.outputResolutionIndex = Math.max(0, state.outputResolutionIndex - 1);
    state.outputWidth = OUTPUT_RESOLUTIONS[state.outputResolutionIndex].width;
    state.outputHeight = OUTPUT_RESOLUTIONS[state.outputResolutionIndex].height;
    applyOutputSize();
});
let recorder = null;
window.addEventListener("toggleRecord", () => {
    if (!recorder) {
        recorder = new Recorder(document.getElementById("master-layer"));
    }
    const bar = document.querySelector(".statusbar");
    if (recorder.recording) {
        recorder.stop();
        if (bar && bar.children[3]) {
            bar.children[3].innerText = "REC: OFF";
        }
    } else {
        recorder.start();
        if (bar && bar.children[3]) {
            bar.children[3].innerText = "REC: ON";
        }
    }
});