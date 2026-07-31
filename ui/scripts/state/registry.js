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
// Exported (not just used internally by getAllRealEntries) so callers that
// just created a layer can compute its entry id without duplicating the
// "kind:id" format - see setSelectedVideoEntry below.
export function toEntry(layer) {
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
// Written only by setSelectedVideoEntry below, right after a video/image/
// camera layer is created - this is what lets the status bar and preview
// panel track the layer that was actually just added instead of always
// falling back to whichever one happens to be first in the registry.
export function selectedVideoEntry() {
    const registry = getVideoRegistry();
    return registry.find(entry => entry.id === state.selectedVideoId) || registry[0] || EMPTY_ENTRY;
}
// Marks a freshly created layer as "the selected one" so selectedVideoEntry
// can find it by id instead of silently defaulting to registry[0].
export function setSelectedVideoEntry(layer) {
    state.selectedVideoId = toEntry(layer).id;
}
// scope is accepted (not silently dropped) for parity with reportSelection's
// call sites, even though there is currently only one scope ("video") to
// distinguish - keeps this from re-becoming an arity mismatch if a second
// scope is ever introduced.
export function scopedEntry(scope) {
    return selectedVideoEntry();
}
export function resolveMaskSourceLabel(sourceId) {
    if (!sourceId || sourceId === "none") {
        return "NONE";
    }
    const match = getAllRealEntries().find(entry => entry.id === sourceId);
    return match ? match.label : "NONE";
}
