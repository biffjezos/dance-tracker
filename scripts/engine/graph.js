/*
==================================================
GRAPH REBUILD
==================================================
*/
import {
    getWasmApp
} from "../core/wasm.js";
import {
    getAllRealEntries,
    scopedEntry
} from "./registry.js";
import {
    state
} from "./state.js";
let outputNodeId = null;
let cachedContentIds = new Map();
let cachedWiredIds = new Map();

function buildVideoContent(layer) {
    const wasmApp = getWasmApp();
    if (!wasmApp || !layer.videoEl) return null;
    return wasmApp.add_video_source(layer.videoEl);
}

function buildMaskContent(layer, contentIds) {
    const wasmApp = getWasmApp();
    const s = layer.settings;
    const sourceContentId = contentIds.get(s.source);
    if (sourceContentId === undefined || sourceContentId === null) {
        return null;
    }
    if (s.mode === "keying") {
        return wasmApp.add_chroma(sourceContentId, s.keyColour.r, s.keyColour.g, s.keyColour.b, s.threshold, s.fill ===
            "video", s.colour.r, s.colour.g, s.colour.b);
    }
    const differenceId = wasmApp.add_difference(sourceContentId, s.threshold, s.fill === "video", s.colour.r, s.colour
        .g, s.colour.b);
    if (layer.pendingCapture) {
        wasmApp.capture_background(differenceId);
    }
    layer.lastDifferenceNodeId = differenceId;
    return differenceId;
}

function buildRingsContent(layer) {
    const wasmApp = getWasmApp();
    return wasmApp.add_rings(layer.settings.count, layer.settings.ringsPerGroup, layer.settings.spacing, layer.settings
        .size, layer.settings.width, layer.settings.colours);
}

function buildGhostContent(layer, contentIds) {
    const wasmApp = getWasmApp();
    const sourceContentId = contentIds.get(layer.settings.applyToMask);
    if (sourceContentId === undefined || sourceContentId === null) {
        return null;
    }
    return wasmApp.add_ghost(sourceContentId, layer.settings.count, layer.settings.alpha, layer.settings.delayTicks);
}

function buildTextContent(layer) {
    const wasmApp = getWasmApp();
    if (!layer.settings.content.trim()) {
        return null;
    }
    return wasmApp.add_text(layer.settings.content, layer.settings.colour, layer.settings.size);
}

function buildCompositeContent(layer, contentIds, wiredIds) {
    const wasmApp = getWasmApp();
    const s = layer.settings;
    if (s.foreground === "none" || s.background === "none") {
        return null;
    }
    const fgId = wiredIds.get(s.foreground) ?? contentIds.get(s.foreground);
    const bgId = wiredIds.get(s.background) ?? contentIds.get(s.background);
    if (fgId === undefined || bgId === undefined) {
        return null;
    }
    return wasmApp.add_compose(fgId, bgId, s.blendMode);
}
export function rebuildGraph() {
    const wasmApp = getWasmApp();
    if (!wasmApp) return;
    const all = getAllRealEntries();
    const contentIds = new Map();
    all.forEach(entry => {
        let id = null;
        if (entry.kind === "video") {
            id = buildVideoContent(entry.layer);
        } else if (entry.kind === "rings") {
            id = buildRingsContent(entry.layer);
        } else if (entry.kind === "text") {
            id = buildTextContent(entry.layer);
        }
        if (id !== null && id !== undefined) {
            contentIds.set(entry.id, id);
        }
    });
    all.forEach(entry => {
        if (entry.kind !== "standaloneMask") {
            return;
        }
        const id = buildMaskContent(entry.layer, contentIds);
        if (id !== null && id !== undefined) {
            contentIds.set(entry.id, id);
        }
    });
    all.forEach(entry => {
        if (entry.kind !== "ghost") {
            return;
        }
        const id = buildGhostContent(entry.layer, contentIds);
        if (id !== null && id !== undefined) {
            contentIds.set(entry.id, id);
        }
    });
    const wiredIds = new Map();
    all.forEach(entry => {
        let id = contentIds.get(entry.id);
        if (id === undefined) {
            return;
        }
        const s = entry.layer.settings;
        if (s.maskedBy.source !== "none") {
            const maskId = contentIds.get(s.maskedBy.source);
            if (maskId !== undefined) {
                id = wasmApp.add_apply_mask(id, maskId, s.maskedBy.channel);
            }
        }
        wiredIds.set(entry.id, id);
    });
    all.forEach(entry => {
        if (entry.kind !== "composite") {
            return;
        }
        const id = buildCompositeContent(entry.layer, contentIds, wiredIds);
        if (id !== null && id !== undefined) {
            wiredIds.set(entry.id, id);
        }
    });
    all.forEach(entry => {
        if (entry.kind !== "composite") {
            return;
        }
        let id = wiredIds.get(entry.id);
        if (id === undefined) {
            return;
        }
        const s = entry.layer.settings;
        if (s.maskedBy.source !== "none") {
            const maskId = wiredIds.get(s.maskedBy.source) ?? contentIds.get(s.maskedBy.source);
            if (maskId !== undefined) {
                id = wasmApp.add_apply_mask(id, maskId, s.maskedBy.channel);
            }
        }
        wiredIds.set(entry.id, id);
    });
    cachedContentIds = contentIds;
    cachedWiredIds = wiredIds;
    updateOutputNodeId();
    state.videoLayers.forEach(layer => {
        layer.pendingCapture = false;
    });
    state.maskLayers.forEach(layer => {
        layer.pendingCapture = false;
    });
}
export function updateOutputNodeId() {
    if (state.outputEntryId === null) {
        outputNodeId = null;
        return;
    }
    const currentId = cachedWiredIds.get(state.outputEntryId);
    outputNodeId = currentId !== undefined ? currentId : null;
}
export function getOutputNodeId() {
    return outputNodeId;
}
export function currentPreviewContentId() {
    const entry = scopedEntry(lastPreviewScope);
    if (!entry.id) {
        return null;
    }
    if (entry.kind === "standaloneMask") {
        return cachedContentIds.get(entry.layer.settings.source);
    }
    return cachedContentIds.get(entry.id);
}