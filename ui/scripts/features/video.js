/*
==================================================
INPUT: CAMERA / VIDEO FILE
==================================================
*/
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
    if (originalVideoLayer) return originalVideoLayer;

    const camera = getCamera();
    if (!camera) return null;

    originalVideoLayer = {
        id: "original",
        number: null,
        name: null,
        videoEl: camera.getVideo(),
        settings: defaultUniversalSettings()
    };

    return originalVideoLayer;
}

/**
 * Extract a frame from a video element as RGBA pixel data
 * @param {HTMLVideoElement} video - The video element
 * @param {number} width - Target width
 * @param {number} height - Target height
 * @returns {Promise<{pixels: Uint8Array, width: number, height: number}>}
 */
async function extractVideoFrame(video, width, height) {
    return new Promise((resolve, reject) => {
        const canvas = document.createElement("canvas");
        const ctx = canvas.getContext("2d");

        if (!ctx) {
            reject(new Error("Could not create canvas context"));
            return;
        }

        try {
            // Set canvas dimensions
            canvas.width = width;
            canvas.height = height;

            // Draw video frame onto canvas
            ctx.drawImage(video, 0, 0, width, height);

            // Get pixel data
            const imageData = ctx.getImageData(0, 0, canvas.width, canvas.height);
            
            resolve({
                pixels: imageData.data,
                width: canvas.width,
                height: canvas.height
            });
        } catch (error) {
            reject(error);
        }
    });
}

// Handle the open_video_picker action from menu
window.addEventListener("menuOperation", e => {
    if (e.detail !== "open_video_picker") return;

    // Create file input element for video
    const input = document.createElement("input");
    input.type = "file";
    input.accept = "video/*";
    input.style.display = "none";

    // Handle file selection
    input.addEventListener("change", async (event) => {
        const file = event.target.files?.[0];
        if (!file) return;

        try {
            // Create a video element to load and play the video
            const video = document.createElement("video");
            video.muted = true;
            video.loop = true;
            video.playsInline = true;
            video.style.display = "none";

            document.body.appendChild(video);

            video.src = URL.createObjectURL(file);
            
            // Wait for metadata to be loaded to get video dimensions
            await new Promise((resolve) => {
                video.onloadedmetadata = resolve;
            });
            
            // Play the video
            await video.play();
            
            // Wait for the first frame to be ready
            await new Promise(resolve => setTimeout(resolve, 100));

            // Create video source node in WASM
            const wasmApp = getWasmApp();
            if (!wasmApp) {
                console.error("WASM app not initialized");
                return;
            }

            // Create the node
            const nodeId = wasmApp.create_video_source_node();
            
            // Extract the first frame from the video
            const frameData = await extractVideoFrame(video, video.videoWidth, video.videoHeight);
            
            // Set the frame data on the node
            wasmApp.set_video_on_node(nodeId, new Uint8Array(frameData.pixels), frameData.width, frameData.height);
            
            // Create a layer for this video
            const number = state.nextVideoNumber++;
            const layer = {
                id: "video-" + number,
                number,
                name: "VIDEO " + number,
                videoNodeId: nodeId,
                settings: defaultUniversalSettings()
            };

            state.videoLayers.push(layer);
            state.transportPlaying = true;

            rebuildGraph();
            reportSelection("video");

        } catch (error) {
            console.error("Error loading video:", error);
        } finally {
            // Clean up
            document.body.removeChild(input);
        }
    });

    // Add to DOM and trigger click
    document.body.appendChild(input);
    input.click();
});

// Handle the open_camera_stream action from menu
window.addEventListener("menuOperation", e => {
    if (e.detail !== "open_camera_stream") return;

    const camera = getCamera();
    if (!camera) return;

    state.cameraOn = true;

    const layer = getOriginalVideoLayer();
    if (!layer) return;

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

window.addEventListener("toggleCamera", () => {
    const camera = getCamera();
    if (!camera) return;

    state.cameraOn = !state.cameraOn;

    if (state.cameraOn) {
        const layer = getOriginalVideoLayer();
        if (!layer) return;

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

window.addEventListener("loadVideoFile", e => {
    const video = document.createElement("video");
    video.muted = true;
    video.loop = true;
    video.playsInline = true;
    video.style.display = "none";

    document.body.appendChild(video);

    video.src = URL.createObjectURL(e.detail.file);
    video.play();

    const number = state.nextVideoNumber++;

    const layer = {
        id: "video-" + number,
        number,
        name: "VIDEO " + number,
        videoEl: video,
        settings: defaultUniversalSettings()
    };

    state.videoLayers.push(layer);
    state.transportPlaying = true;

    rebuildGraph();
    reportSelection("video");
});

window.addEventListener("addVideoLayer", e => {
    const video = document.createElement("video");

    video.muted = true;
    video.loop = true;
    video.playsInline = true;
    video.style.display = "none";

    document.body.appendChild(video);

    video.src = URL.createObjectURL(e.detail.file);
    video.play();

    const number = state.nextVideoNumber++;

    const layer = {
        id: "video-" + number,
        number,
        name: "VIDEO " + number,
        videoEl: video,
        settings: defaultUniversalSettings()
    };

    state.videoLayers.push(layer);
    state.transportPlaying = true;

    rebuildGraph();
    reportSelection("video");
});
