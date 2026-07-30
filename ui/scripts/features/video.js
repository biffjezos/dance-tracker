// features/video.js
import {
    state,
    nextNumber
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

/*
    Attach a live HTMLVideoElement (a decoded file, or a camera stream) to a
    freshly created node of the given operation id. The scratch canvas is
    where the pixel source draws each frame before reading it back.
*/
function createLiveSourceNode(wasmApp, operationId, video) {
    const nodeId = wasmApp.create_node(operationId);
    const scratchCanvas = document.createElement("canvas");
    scratchCanvas.width = video.videoWidth;
    scratchCanvas.height = video.videoHeight;
    wasmApp.set_pixel_source_on_node(nodeId, video, scratchCanvas);
    return nodeId;
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
            const nodeId = createLiveSourceNode(wasmApp, "video_source", video);
            const number = nextNumber("video");
            const layer = {
                id: "video-" + number,
                number,
                name: "VIDEO " + number,
                nodeId: nodeId,
                kind: "video",
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
function activateCamera() {
    const camera = getCamera();
    if (!camera) {
        return null;
    }
    const wasmApp = getWasmApp();
    if (!wasmApp) {
        return null;
    }
    const video = camera.getVideo();
    const number = nextNumber("camera");
    const layer = {
        id: "camera-" + number,
        number,
        name: "CAMERA " + number,
        kind: "video",
        videoEl: video,
        settings: defaultUniversalSettings()
    };
    // The camera node needs an actual frame size before it can be created,
    // which only exists once the stream has loaded metadata.
    const attach = () => {
        layer.nodeId = createLiveSourceNode(wasmApp, "camera_source", video);
        state.videoLayers.push(layer);
        rebuildGraph();
        reportSelection("video");
    };
    if (video.videoWidth && video.videoHeight) {
        attach();
    } else {
        video.addEventListener("loadedmetadata", attach, {
            once: true
        });
    }
    return layer;
}

window.addEventListener("menuOperation", e => {
    if (e.detail !== "open_camera_stream") {
        return;
    }
    const camera = getCamera();
    if (!camera) {
        return;
    }
    state.cameraOn = true;
    if (!state.cameraActivated) {
        state.cameraActivated = true;
        activateCamera();
    }
    state.transportPlaying = true;
    camera.start();
});

window.addEventListener("toggleCamera", () => {
    const camera = getCamera();
    if (!camera) {
        return;
    }
    state.cameraOn = !state.cameraOn;
    if (state.cameraOn) {
        if (!state.cameraActivated) {
            state.cameraActivated = true;
            activateCamera();
        }
        state.transportPlaying = true;
        camera.start();
    } else {
        camera.stop();
    }
});
