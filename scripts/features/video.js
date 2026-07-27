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
} from "../engine/registry.js";
const camera = getCamera();
const originalVideoLayer = {
    id: "original",
    number: null,
    name: null,
    videoEl: camera.getVideo(),
    settings: defaultUniversalSettings()
};
window.addEventListener("toggleCamera", () => {
    state.cameraOn = !state.cameraOn;
    if (state.cameraOn) {
        if (!state.cameraActivated) {
            state.cameraActivated = true;
            originalVideoLayer.number = state.nextVideoNumber++;
            originalVideoLayer.name = "VIDEO " + originalVideoLayer.number;
            state.videoLayers.push(originalVideoLayer);
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
    rebuildGraph();
    reportSelection("video");
});
