/*
==================================================
TRANSPORT
==================================================
*/
import {
    state
} from "../engine/state.js";
import {
    getWasmApp
} from "../core/wasm.js";
import {
    selectedVideoEntry
} from "../engine/registry.js";
import {
    getCamera
} from "../engine/camera.js";

function currentTransportVideo() {
    const entry = selectedVideoEntry();
    if (entry.layer && entry.layer.videoEl) {
        return entry.layer.videoEl;
    }
    const camera = getCamera();
    return camera ? camera.getVideo() : null;
}

function hasVideoFile() {
    const video = currentTransportVideo();
    return video && isFinite(video.duration) && video.duration > 0;
}
window.addEventListener("transportPlayStop", () => {
    const wasmApp = getWasmApp();
    if (!wasmApp) return;
    const video = currentTransportVideo();
    if (!video) return;
    const hasFile = hasVideoFile();
    state.transportPlaying = hasFile ? video.paused : !state.transportPlaying;
    if (state.transportPlaying) {
        if (hasFile) {
            wasmApp.play(video);
        }
    } else {
        if (hasFile) {
            wasmApp.stop(video);
        }
    }
});

function seekBy(seconds) {
    const wasmApp = getWasmApp();
    if (!wasmApp) return;
    const video = currentTransportVideo();
    if (!video || !hasVideoFile()) return;
    if (seconds >= 0) {
        wasmApp.forward(video, seconds);
    } else {
        wasmApp.rewind(video, -seconds);
    }
}
window.addEventListener("transportMinuteUp",
    () => seekBy(60));
window.addEventListener("transportMinuteDown",
    () => seekBy(-60));
window.addEventListener("transportSecondUp",
    () => seekBy(1));
window.addEventListener("transportSecondDown",
    () => seekBy(-1));
window.addEventListener("transportFrameUp",
    () => seekBy(1 / 30));
window.addEventListener("transportFrameDown",
    () => seekBy(-1 / 30));