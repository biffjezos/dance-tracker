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
    // Every create_node-backed operation (shuffle, and any future one)
    // lives in this one generic array, tagged with its own kind.
    state.nodes.forEach(layer => list.push({
        id: layer.kind + ":" + layer.id,
        label: layer.name,
        kind: layer.kind,
        layer
    }));
    return list;
}
export function getVideoRegistry() {
    // Every real node is a candidate source/preview target by default - new
    // kinds are included automatically, no filter list to maintain here.
    return getAllRealEntries();
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
