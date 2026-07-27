/*
==================================================
GENERATE: RINGS / GHOST / TEXT
==================================================
*/
import {
    state
} from "../engine/state.js";
import {
    selectedVideoEntry,
    getAllRealEntries,
    defaultUniversalSettings
} from "../engine/registry.js";
import {
    rebuildGraph
} from "../engine/graph.js";
import {
    reportSelection
} from "../engine/status.js";

function addRingsLayer() {
    const number = state.nextRingsNumber++;
    const layer = {
        id: "rings-" + number,
        number,
        name: "RINGS " + number,
        settings: {
            ...defaultUniversalSettings(),
            count: 2,
            ringsPerGroup: 8,
            spacing: 14,
            size: 20,
            width: 6,
            colours: ["rgb(255,0,255)", "rgb(0,255,80)"]
        }
    };
    state.ringsLayers.push(layer);
    return layer;
}
window.addEventListener("addRingsLayer", () => {
    addRingsLayer();
    rebuildGraph();
    reportSelection("video");
});
window.addEventListener("ringCountUp", () => {
    const s = selectedVideoEntry().layer.settings;
    if (s.count !== undefined && s.count < 8) s.count++;
    rebuildGraph();
});
window.addEventListener("ringCountDown", () => {
    const s = selectedVideoEntry().layer.settings;
    if (s.count !== undefined && s.count > 1) s.count--;
    rebuildGraph();
});
window.addEventListener("ringSizeUp", () => {
    const s = selectedVideoEntry().layer.settings;
    if (s.size !== undefined) s.size += 5;
    rebuildGraph();
});
window.addEventListener("ringSizeDown", () => {
    const s = selectedVideoEntry().layer.settings;
    if (s.size !== undefined) s.size = Math.max(5, s.size - 5);
    rebuildGraph();
});
window.addEventListener("ringThicknessUp", () => {
    const s = selectedVideoEntry().layer.settings;
    if (s.width !== undefined) s.width++;
    rebuildGraph();
});
window.addEventListener("ringThicknessDown", () => {
    const s = selectedVideoEntry().layer.settings;
    if (s.width !== undefined) s.width = Math.max(1, s.width - 1);
    rebuildGraph();
});
window.addEventListener("ringColour", e => {
    const s = selectedVideoEntry().layer.settings;
    if (!Array.isArray(s.colours)) return;
    const index = e.detail.ringId - 1;
    while (s.colours.length <= index) s.colours.push("rgb(255,0,255)");
    s.colours[index] = "rgb(" + e.detail.r + "," + e.detail.g + "," + e.detail.b + ")";
    rebuildGraph();
});

function addGhostLayer() {
    const number = state.nextGhostNumber++;
    const layer = {
        id: "ghost-" + number,
        number,
        name: "GHOST " + number,
        settings: {
            ...defaultUniversalSettings(),
            count: 3,
            alpha: 0.45,
            delayTicks: 3,
            applyToMask: "none"
        }
    };
    state.ghostLayers.push(layer);
    return layer;
}
window.addEventListener("addGhostLayer", () => {
    addGhostLayer();
    rebuildGraph();
    reportSelection("video");
});
window.addEventListener("ghostUp", () => {
    const s = selectedVideoEntry().layer.settings;
    if (s.count !== undefined) s.count++;
    rebuildGraph();
});
window.addEventListener("ghostDown", () => {
    const s = selectedVideoEntry().layer.settings;
    if (s.count !== undefined) s.count = Math.max(0, s.count - 1);
    rebuildGraph();
});
window.addEventListener("ghostDelayUp", () => {
    const s = selectedVideoEntry().layer.settings;
    if (s.delayTicks !== undefined) s.delayTicks++;
    rebuildGraph();
});
window.addEventListener("ghostDelayDown", () => {
    const s = selectedVideoEntry().layer.settings;
    if (s.delayTicks !== undefined) s.delayTicks = Math.max(1, s.delayTicks - 1);
    rebuildGraph();
});

function eligibleMaskTargets(excludeId) {
    return getAllRealEntries().filter(entry => entry.id !== excludeId);
}
window.addEventListener("requestApplyToMaskRefresh", e => {
    const entry = selectedVideoEntry();
    if (entry.kind !== "ghost") return;
    const eligible = eligibleMaskTargets(entry.id);
    const match = eligible.find(target => target.id === entry.layer.settings.applyToMask);
    e.detail.label = match ? match.label : (eligible.length ? "NONE" : "NONE AVAILABLE");
    e.detail.options = eligible.map(target => ({
        id: target.id,
        label: target.label
    }));
});
window.addEventListener("setApplyToMask", e => {
    const entry = selectedVideoEntry();
    if (entry.kind !== "ghost") return;
    const eligible = eligibleMaskTargets(entry.id);
    const match = eligible.find(target => target.id === e.detail.id);
    if (!match) return;
    entry.layer.settings.applyToMask = match.id;
    rebuildGraph();
});

function addTextLayer() {
    const number = state.nextTextNumber++;
    const layer = {
        id: "text-" + number,
        number,
        name: "TEXT " + number,
        settings: {
            ...defaultUniversalSettings(),
            content: "",
            colour: "rgb(255,255,255)",
            size: 24
        }
    };
    state.textLayers.push(layer);
    return layer;
}
window.addEventListener("addTextLayer", () => {
    addTextLayer();
    rebuildGraph();
    reportSelection("video");
});
window.addEventListener("setText", e => {
    const s = selectedVideoEntry().layer.settings;
    if (s.content === undefined) return;
    s.content = e.detail.value;
    rebuildGraph();
});
window.addEventListener("textSizeUp", () => {
    const s = selectedVideoEntry().layer.settings;
    if (s.size !== undefined) s.size += 2;
    rebuildGraph();
});
window.addEventListener("textSizeDown", () => {
    const s = selectedVideoEntry().layer.settings;
    if (s.size !== undefined) s.size = Math.max(8, s.size - 2);
    rebuildGraph();
});
window.addEventListener("textColour", e => {
    const s = selectedVideoEntry().layer.settings;
    if (s.colour === undefined) return;
    s.colour = "rgb(" + e.detail.r + "," + e.detail.g + "," + e.detail.b + ")";
    rebuildGraph();
});