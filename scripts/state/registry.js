import {
    state
} from "./state.js";
export function defaultUniversalSettings() {
    return {
        maskedBy: {
            source: "none",
            channel: "alpha"
        }
    };
}
export function getAllRealEntries() {
    const list = [];
    state.videoLayers.forEach(layer => list.push({
        id: "video:" + layer.id,
        label: layer.name,
        kind: "video",
        layer
    }));
    state.maskLayers.forEach(layer => list.push({
        id: "mask:" + layer.id,
        label: layer.name,
        kind: "standaloneMask",
        layer
    }));
    state.compositeLayers.forEach(layer => list.push({
        id: "composite:" + layer.id,
        label: layer.name,
        kind: "composite",
        layer
    }));
    state.ringsLayers.forEach(layer => list.push({
        id: "rings:" + layer.id,
        label: layer.name,
        kind: "rings",
        layer
    }));
    state.ghostLayers.forEach(layer => list.push({
        id: "ghost:" + layer.id,
        label: layer.name,
        kind: "ghost",
        layer
    }));
    state.textLayers.forEach(layer => list.push({
        id: "text:" + layer.id,
        label: layer.name,
        kind: "text",
        layer
    }));
    return list;
}
export function getVideoRegistry() {
    return getAllRealEntries().filter(entry => entry.kind === "video" || entry.kind === "rings" || entry.kind ===
        "ghost" || entry.kind === "text" || entry.kind === "standaloneMask" || entry.kind === "composite");
}
export function getMaskRegistry() {
    return getAllRealEntries().filter(entry => entry.kind === "standaloneMask");
}
export function getCompositeRegistry() {
    return getAllRealEntries().filter(entry => entry.kind === "composite");
}
const EMPTY_ENTRY = {
    label: null,
    kind: null,
    id: null,
    layer: {
        settings: defaultUniversalSettings()
    }
};
export function selectedVideoEntry() {
    const registry = getVideoRegistry();
    return registry.find(entry => entry.id === state.selectedVideoId) || registry[0] || EMPTY_ENTRY;
}
export function selectedMaskEntry() {
    const registry = getMaskRegistry();
    return registry.find(entry => entry.id === state.selectedMaskId) || registry[0] || EMPTY_ENTRY;
}
export function selectedCompositeEntry() {
    const registry = getCompositeRegistry();
    return registry.find(entry => entry.id === state.selectedCompositeId) || registry[0] || EMPTY_ENTRY;
}
export function scopedEntry(scope) {
    if (scope === "mask") return selectedMaskEntry();
    if (scope === "composite") return selectedCompositeEntry();
    return selectedVideoEntry();
}
export function resolveMaskSourceLabel(sourceId) {
    if (!sourceId || sourceId === "none") {
        return "NONE";
    }
    const match = getAllRealEntries().find(entry => entry.id === sourceId);
    return match ? match.label : "NONE";
}