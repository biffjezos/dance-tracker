/*
==================================================
KEY: MASKS
==================================================
*/
import {
    state
} from "../engine/state.js";
import {
    selectedMaskEntry,
    getVideoRegistry,
    defaultUniversalSettings
} from "../state/registry.js";
import {
    rebuildGraph
} from "../engine/graph.js";
import {
    reportSelection
} from "../engine/status.js";
import {
    containFit
} from "../engine/fit.js";

function addMaskLayer() {
    const number = state.nextMaskNumber++;
    const layer = {
        id: "mask-" + number,
        number,
        name: "MASK " + number,
        settings: {
            ...defaultUniversalSettings(),
            mode: "difference",
            threshold: 100,
            keyColour: {
                r: 0,
                g: 255,
                b: 0
            },
            fill: "solid",
            colour: {
                r: 255,
                g: 0,
                b: 255
            },
            source: "none"
        }
    };
    state.maskLayers.push(layer);
    return layer;
}
window.addEventListener("addMaskLayer", () => {
    addMaskLayer();
    rebuildGraph();
    reportSelection("mask");
});
window.addEventListener("maskVideoSourceStep", e => {
    const entry = selectedMaskEntry();
    if (entry.kind !== "standaloneMask") return;
    const ids = ["none", ...getVideoRegistry().filter(v => v.kind === "video").map(v => v.id)];
    let index = ids.indexOf(entry.layer.settings.source);
    if (index < 0) index = 0;
    index = Math.min(Math.max(index + e.detail.direction, 0), ids.length - 1);
    entry.layer.settings.source = ids[index];
    rebuildGraph();
    reportSelection("mask");
});
window.addEventListener("captureLayerBackground", () => {
    const entry = selectedMaskEntry();
    if (entry.kind !== "standaloneMask") return;
    entry.layer.pendingCapture = true;
    rebuildGraph();
});
window.addEventListener("thresholdUp", () => {
    const s = selectedMaskEntry().layer.settings;
    if (!s) return;
    s.threshold += 5;
    rebuildGraph();
});
window.addEventListener("thresholdDown", () => {
    const s = selectedMaskEntry().layer.settings;
    if (!s) return;
    s.threshold = Math.max(0, s.threshold - 5);
    rebuildGraph();
});
window.addEventListener("toggleMatteMode", () => {
    const s = selectedMaskEntry().layer.settings;
    if (!s) return;
    s.mode = s.mode === "difference" ? "keying" : "difference";
    rebuildGraph();
    reportSelection("mask");
});
window.addEventListener("toggleLayerFill", () => {
    const s = selectedMaskEntry().layer.settings;
    if (!s) return;
    s.fill = s.fill === "solid" ? "video" : "solid";
    rebuildGraph();
});
window.addEventListener("layerColour", e => {
    const s = selectedMaskEntry().layer.settings;
    if (!s) return;
    s.colour = {
        r: e.detail.r,
        g: e.detail.g,
        b: e.detail.b
    };
    rebuildGraph();
});
window.addEventListener("bodyKeyColour", e => {
    const s = selectedMaskEntry().layer.settings;
    if (!s) return;
    s.keyColour = {
        r: e.detail.r,
        g: e.detail.g,
        b: e.detail.b
    };
    rebuildGraph();
});
let armedKeyColourPick = false;
window.addEventListener("armKeyColourPicker", () => {
    armedKeyColourPick = true;
    document.getElementById("camera-preview").classList.add("sampling");
});
document.getElementById("camera-preview").addEventListener("click", e => {
    if (!armedKeyColourPick) return;
    armedKeyColourPick = false;
    const source = e.target;
    source.classList.remove("sampling");
    const rect = source.getBoundingClientRect();
    const fit = containFit(source.width, source.height, rect.width, rect.height);
    const x = Math.floor(
        (e.clientX - rect.left - fit.x) / fit.width * source.width);
    const y = Math.floor(
        (e.clientY - rect.top - fit.y) / fit.height * source.height);
    if (x < 0 || y < 0 || x >= source.width || y >= source.height) {
        return;
    }
    const pixel = source.getContext("2d").getImageData(x, y, 1, 1).data;
    window.dispatchEvent(new CustomEvent("bodyKeyColour", {
        detail: {
            r: pixel[0],
            g: pixel[1],
            b: pixel[2]
        }
    }));
});