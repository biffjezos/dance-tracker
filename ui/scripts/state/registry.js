import {
    state
} from "../engine/state.js";
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
    state.videoLayers.forEach(layer => {
        // Check if this is an image layer (has imageNodeId)
        if (layer.imageNodeId !== undefined) {
            list.push({
                id: "image:" + layer.id,
                label: layer.name,
                kind: "image",
                layer
            });
        } else {
            list.push({
                id: "video:" + layer.id,
                label: layer.name,
                kind: "video",
                layer
            });
        }
    });
    state.shuffleLayers.forEach(layer => list.push({
        id: "shuffle:" + layer.id,
        label: layer.name,
        kind: "shuffle",
        layer
    }));
    return list;
}
export function getVideoRegistry() {
    return getAllRealEntries().filter(entry => entry.kind === "video" || entry.kind === "image" || entry.kind ===
        "shuffle");
}
export function getShuffleRegistry() {
    return getAllRealEntries().filter(entry => entry.kind === "shuffle");
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
export function scopedEntry() {
    return selectedVideoEntry();
}
export function resolveMaskSourceLabel(sourceId) {
    if (!sourceId || sourceId === "none") {
        return "NONE";
    }
    const match = getAllRealEntries().find(entry => entry.id === sourceId);
    return match ? match.label : "NONE";
}
