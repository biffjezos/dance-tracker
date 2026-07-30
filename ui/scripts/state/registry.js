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
function toEntry(layer) {
    return {
        id: layer.kind + ":" + layer.id,
        label: layer.name,
        kind: layer.kind,
        layer
    };
}
export function getAllRealEntries() {
    // videoLayers (video/image/camera - created via their own bespoke
    // external-resource flow) and nodes (any create_node-backed operation,
    // e.g. shuffle) are separate arrays only because of how they're
    // created; every entry they produce has the same generic shape.
    return [
        ...state.videoLayers.map(toEntry),
        ...state.nodes.map(toEntry)
    ];
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
