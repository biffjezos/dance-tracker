// features/video.js
import {
    state
} from "../engine/state.js";
import {
    getCamera
} from "../engine/camera.js";
import {
    rebuildGraph
} from "../engine/graph.js";
import {
    reportSelection
} from "../engine/status.js";
import {
    defaultUniversalSettings
} from "../state/registry.js";
import {
    getWasmApp
} from "../core/wasm.js";
let originalVideoLayer = null;

function getOriginalVideoLayer() {
    if (originalVideoLayer) {
        return originalVideoLayer;
    }
    const camera = getCamera();
    if (!camera) {
        return null;
    }
    originalVideoLayer = {
        id: "original",
        number: null,
        name: null,
        videoEl: camera.getVideo(),
        settings: defaultUniversalSettings()
    };
    return originalVideoLayer;
}
/*
    Open video file

    Browser owns decoding.
    Rust receives the HTMLVideoElement
    and pulls frames through WASM.
*/
window.addEventListener("menuOperation", e => {
    if (e.detail !== "open_video_picker") {
        return;
    }
    const input = document.createElement("input");
    input.type = "file";
    input.accept = "video/*";
    input.style.display = "none";
    input.addEventListener("change", async event => {
        const file = event.target.files?.[0];
        if (!file) {
            return;
        }
        try {
            const video = document.createElement("video");
            video.muted = true;
            video.loop = true;
            video.playsInline = true;
            video.style.display = "none";
            document.body.appendChild(video);
            video.src = URL.createObjectURL(file);
            await new Promise(resolve => {
                video.onloadedmetadata = resolve;
            });
            await video.play();
            const wasmApp = getWasmApp();
            if (!wasmApp) {
                console.error("WASM app not initialized");
                return;
            }
            const nodeId = wasmApp.create_video_source_node();
            const scratchCanvas = document.createElement("canvas");
            scratchCanvas.width = video.videoWidth;
            scratchCanvas.height = video.videoHeight;
            wasmApp.set_video_element_on_node(nodeId, video, scratchCanvas);
            const number = state.nextVideoNumber++;
            const layer = {
                id: "video-" + number,
                number,
                name: "VIDEO " + number,
                videoNodeId: nodeId,
                videoEl: video,
                settings: defaultUniversalSettings()
            };
            state.videoLayers.push(layer);
            state.transportPlaying = true;
            rebuildGraph();
            reportSelection("video");
        } catch (error) {
            console.error("Error loading video:", error);
        } finally {
            input.remove();
        }
    });
    document.body.appendChild(input);
    input.click();
});
// Camera stream handling
window.addEventListener("menuOperation", e => {
    if (e.detail !== "open_camera_stream") {
        return;
    }
    const camera = getCamera();
    if (!camera) {
        return;
    }
    state.cameraOn = true;
    const layer = getOriginalVideoLayer();
    if (!layer) {
        return;
    }
    if (!state.cameraActivated) {
        state.cameraActivated = true;
        layer.number = state.nextVideoNumber++;
        layer.name = "VIDEO " + layer.number;
        state.videoLayers.push(layer);
        rebuildGraph();
        reportSelection("video");
    }
    state.transportPlaying = true;
    camera.start();
});
window.addEventListener("toggleCamera",
    () => {
        const camera = getCamera();
        if (!camera) {
            return;
        }
        state.cameraOn = !state.cameraOn;
        if (state.cameraOn) {
            const layer = getOriginalVideoLayer();
            if (!layer) {
                return;
            }
            if (!state.cameraActivated) {
                state.cameraActivated = true;
                layer.number = state.nextVideoNumber++;
                layer.name = "VIDEO " + layer.number;
                state.videoLayers.push(layer);
                rebuildGraph();
                reportSelection("video");
            }
            state.transportPlaying = true;
            camera.start();
        } else {
            camera.stop();
        }
    });
// External video loading
window.addEventListener("loadVideoFile", async e => {
    const video = document.createElement("video");
    video.muted = true;
    video.loop = true;
    video.playsInline = true;
    video.style.display = "none";
    document.body.appendChild(video);
    video.src = URL.createObjectURL(e.detail.file);
    await video.play();
    const wasmApp = getWasmApp();
    if (!wasmApp) {
        return;
    }
    const nodeId = wasmApp.create_video_source_node();
    const scratchCanvas = document.createElement("canvas");
    scratchCanvas.width = video.videoWidth;
    scratchCanvas.height = video.videoHeight;
    wasmApp.set_video_element_on_node(nodeId, video, scratchCanvas);
    const number = state.nextVideoNumber++;
    const layer = {
        id: "video-" + number,
        number,
        name: "VIDEO " + number,
        videoNodeId: nodeId,
        videoEl: video,
        settings: defaultUniversalSettings()
    };
    state.videoLayers.push(layer);
    state.transportPlaying = true;
    rebuildGraph();
    reportSelection("video");
});
// Add video layer
window.addEventListener("addVideoLayer", async e => {
    const video = document.createElement("video");
    video.muted = true;
    video.loop = true;
    video.playsInline = true;
    video.style.display = "none";
    document.body.appendChild(video);
    video.src = URL.createObjectURL(e.detail.file);
    await video.play();
    const wasmApp = getWasmApp();
    if (!wasmApp) {
        return;
    }
    const nodeId = wasmApp.create_video_source_node();
    const scratchCanvas = document.createElement("canvas");
    scratchCanvas.width = video.videoWidth;
    scratchCanvas.height = video.videoHeight;
    wasmApp.set_video_element_on_node(nodeId, video, scratchCanvas);
    const number = state.nextVideoNumber++;
    const layer = {
        id: "video-" + number,
        number,
        name: "VIDEO " + number,
        videoNodeId: nodeId,
        videoEl: video,
        settings: defaultUniversalSettings()
    };
    state.videoLayers.push(layer);
    state.transportPlaying = true;
    rebuildGraph();
    reportSelection("video");
}); // features/video.js
import {
    state
} from "../engine/state.js";
import {
    getCamera
} from "../engine/camera.js";
import {
    rebuildGraph
} from "../engine/graph.js";
import {
    reportSelection
} from "../engine/status.js";
import {
    defaultUniversalSettings
} from "../state/registry.js";
import {
    getWasmApp
} from "../core/wasm.js";
let originalVideoLayer = null;

function getOriginalVideoLayer() {
    if (originalVideoLayer) {
        return originalVideoLayer;
    }
    const camera = getCamera();
    if (!camera) {
        return null;
    }
    originalVideoLayer = {
        id: "original",
        number: null,
        name: null,
        videoEl: camera.getVideo(),
        settings: defaultUniversalSettings()
    };
    return originalVideoLayer;
}
/*
    Open video file

    Browser owns decoding.
    Rust receives the HTMLVideoElement
    and pulls frames through WASM.
*/
window.addEventListener("menuOperation", e => {
    if (e.detail !== "open_video_picker") {
        return;
    }
    const input = document.createElement("input");
    input.type = "file";
    input.accept = "video/*";
    input.style.display = "none";
    input.addEventListener("change", async event => {
        const file = event.target.files?.[0];
        if (!file) {
            return;
        }
        try {
            const video = document.createElement("video");
            video.muted = true;
            video.loop = true;
            video.playsInline = true;
            video.style.display = "none";
            document.body.appendChild(video);
            video.src = URL.createObjectURL(file);
            await new Promise(resolve => {
                video.onloadedmetadata = resolve;
            });
            await video.play();
            const wasmApp = getWasmApp();
            if (!wasmApp) {
                console.error("WASM app not initialized");
                return;
            }
            const nodeId = wasmApp.create_video_source_node();
            const scratchCanvas = document.createElement("canvas");
            scratchCanvas.width = video.videoWidth;
            scratchCanvas.height = video.videoHeight;
            wasmApp.set_video_element_on_node(nodeId, video, scratchCanvas);
            const number = state.nextVideoNumber++;
            const layer = {
                id: "video-" + number,
                number,
                name: "VIDEO " + number,
                videoNodeId: nodeId,
                videoEl: video,
                settings: defaultUniversalSettings()
            };
            state.videoLayers.push(layer);
            state.transportPlaying = true;
            rebuildGraph();
            reportSelection("video");
        } catch (error) {
            console.error("Error loading video:", error);
        } finally {
            input.remove();
        }
    });
    document.body.appendChild(input);
    input.click();
});
// Camera stream handling
window.addEventListener("menuOperation", e => {
    if (e.detail !== "open_camera_stream") {
        return;
    }
    const camera = getCamera();
    if (!camera) {
        return;
    }
    state.cameraOn = true;
    const layer = getOriginalVideoLayer();
    if (!layer) {
        return;
    }
    if (!state.cameraActivated) {
        state.cameraActivated = true;
        layer.number = state.nextVideoNumber++;
        layer.name = "VIDEO " + layer.number;
        state.videoLayers.push(layer);
        rebuildGraph();
        reportSelection("video");
    }
    state.transportPlaying = true;
    camera.start();
});
window.addEventListener("toggleCamera",
    () => {
        const camera = getCamera();
        if (!camera) {
            return;
        }
        state.cameraOn = !state.cameraOn;
        if (state.cameraOn) {
            const layer = getOriginalVideoLayer();
            if (!layer) {
                return;
            }
            if (!state.cameraActivated) {
                state.cameraActivated = true;
                layer.number = state.nextVideoNumber++;
                layer.name = "VIDEO " + layer.number;
                state.videoLayers.push(layer);
                rebuildGraph();
                reportSelection("video");
            }
            state.transportPlaying = true;
            camera.start();
        } else {
            camera.stop();
        }
    });
// External video loading
window.addEventListener("loadVideoFile", async e => {
    const video = document.createElement("video");
    video.muted = true;
    video.loop = true;
    video.playsInline = true;
    video.style.display = "none";
    document.body.appendChild(video);
    video.src = URL.createObjectURL(e.detail.file);
    await video.play();
    const wasmApp = getWasmApp();
    if (!wasmApp) {
        return;
    }
    const nodeId = wasmApp.create_video_source_node();
    const scratchCanvas = document.createElement("canvas");
    scratchCanvas.width = video.videoWidth;
    scratchCanvas.height = video.videoHeight;
    wasmApp.set_video_element_on_node(nodeId, video, scratchCanvas);
    const number = state.nextVideoNumber++;
    const layer = {
        id: "video-" + number,
        number,
        name: "VIDEO " + number,
        videoNodeId: nodeId,
        videoEl: video,
        settings: defaultUniversalSettings()
    };
    state.videoLayers.push(layer);
    state.transportPlaying = true;
    rebuildGraph();
    reportSelection("video");
});
// Add video layer
window.addEventListener("addVideoLayer", async e => {
    const video = document.createElement("video");
    video.muted = true;
    video.loop = true;
    video.playsInline = true;
    video.style.display = "none";
    document.body.appendChild(video);
    video.src = URL.createObjectURL(e.detail.file);
    await video.play();
    const wasmApp = getWasmApp();
    if (!wasmApp) {
        return;
    }
    const nodeId = wasmApp.create_video_source_node();
    const scratchCanvas = document.createElement("canvas");
    scratchCanvas.width = video.videoWidth;
    scratchCanvas.height = video.videoHeight;
    wasmApp.set_video_element_on_node(nodeId, video, scratchCanvas);
    const number = state.nextVideoNumber++;
    const layer = {
        id: "video-" + number,
        number,
        name: "VIDEO " + number,
        videoNodeId: nodeId,
        videoEl: video,
        settings: defaultUniversalSettings()
    };
    state.videoLayers.push(layer);
    state.transportPlaying = true;
    rebuildGraph();
    reportSelection("video");
});