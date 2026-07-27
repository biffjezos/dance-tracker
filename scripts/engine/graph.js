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
} from "../state/registry.js";
import {
    state
} from "./state.js";
let outputNodeId = null;
let cachedContentIds = new Map();
let cachedWiredIds = new Map();
let lastPreviewScope = "video";

function buildVideoContent(layer) {
    const wasmApp = getWasmApp();
    if (!wasmApp || !layer.videoEl) return null;
    return wasmApp.add_video_source(layer.videoEl);
}

function buildMaskContent(layer, contentIds) {
    const wasmApp = getWasmApp();
    const s = layer.settings;
    const sourceId = contentIds.get(s.source);
    if (sourceId === undefined) return null;
    if (s.mode === "keying") {
        return wasmApp.add_chroma(sourceId, s.keyColour.r, s.keyColour.g, s.keyColour.b, s.threshold, s.fill ===
            "video", s.colour.r, s.colour.g, s.colour.b);
    }
    const differenceId = wasmApp.add_difference(sourceId, s.threshold, s.fill === "video", s.colour.r, s.colour.g, s
        .colour.b);
    if (layer.pendingCapture) {
        wasmApp.capture_background(differenceId);
    }
    return differenceId;
}

function buildRingsContent(layer) {
    const wasmApp = getWasmApp();
    return wasmApp.add_rings(layer.settings.count, layer.settings.ringsPerGroup, layer.settings.spacing, layer.settings
        .size, layer.settings.width, layer.settings.colours);
}

function buildGhostContent(layer, ids) {
    const wasmApp = getWasmApp();
    const source = ids.get(layer.settings.applyToMask);
    if (source === undefined) return null;
    return wasmApp.add_ghost(source, layer.settings.count, layer.settings.alpha, layer.settings.delayTicks);
}

function buildTextContent(layer) {
    const wasmApp = getWasmApp();
    if (!layer.settings.content.trim()) return null;
    return wasmApp.add_text(layer.settings.content, layer.settings.colour, layer.settings.size);
}

function buildCompositeContent(layer, contentIds, wiredIds) {
    const wasmApp = getWasmApp();
    const s = layer.settings;
    if (s.foreground === "none" || s.background === "none") return null;
    const fg = wiredIds.get(s.foreground) ?? contentIds.get(s.foreground);
    const bg = wiredIds.get(s.background) ?? contentIds.get(s.background);
    if (fg === undefined || bg === undefined) return null;
    return wasmApp.add_compose(fg, bg, s.blendMode);
}
export function rebuildGraph() {
    const wasmApp = getWasmApp();
    if (!wasmApp) return;
    const all = getAllRealEntries();
    const contentIds = new Map();
    /*
    FIRST PASS:
    Build primitive generators.
    */
    all.forEach(entry => {
        let id = null;
        if (entry.kind === "video") id = buildVideoContent(entry.layer);
        if (entry.kind === "rings") id = buildRingsContent(entry.layer);
        if (entry.kind === "text") id = buildTextContent(entry.layer);
        if (id !== null) contentIds.set(entry.id, id);
    });
    /*
    SECOND PASS:
    Build masks from primitive sources.
    */
    all.forEach(entry => {
        if (entry.kind !== "standaloneMask") return;
        const id = buildMaskContent(entry.layer, contentIds);
        if (id !== null) contentIds.set(entry.id, id);
    });
    /*
    THIRD PASS:
    Apply universal MASKED BY.
    */
    const wiredIds = new Map();
    all.forEach(entry => {
        let id = contentIds.get(entry.id);
        if (id === undefined) return;
        const mask = entry.layer.settings.maskedBy;
        if (mask && mask.source !== "none") {
            const maskId = contentIds.get(mask.source);
            if (maskId !== undefined) {
                id = wasmApp.add_apply_mask(id, maskId, mask.channel);
            }
        }
        wiredIds.set(entry.id, id);
    });
    /*
    FOURTH PASS:
    Ghosts now see final wired nodes.
    */
    all.forEach(entry => {
        if (entry.kind !== "ghost") return;
        const id = buildGhostContent(entry.layer, wiredIds);
        if (id !== null) wiredIds.set(entry.id, id);
    });
    /*
    FIFTH PASS:
    Composite only here.
    */
    all.forEach(entry => {
        if (entry.kind !== "composite") return;
        const id = buildCompositeContent(entry.layer, contentIds, wiredIds);
        if (id !== null) wiredIds.set(entry.id, id);
    });
    /*
    FINAL:
    Apply masks to composites.
    */
    all.forEach(entry => {
        if (entry.kind !== "composite") return;
        let id = wiredIds.get(entry.id);
        if (id === undefined) return;
        const mask = entry.layer.settings.maskedBy;
        if (mask && mask.source !== "none") {
            const maskId = wiredIds.get(mask.source) ?? contentIds.get(mask.source);
            if (maskId !== undefined) {
                id = wasmApp.add_apply_mask(id, maskId, mask.channel);
            }
        }
        wiredIds.set(entry.id, id);
    });
    cachedContentIds = contentIds;
    cachedWiredIds = wiredIds;
    updateOutputNodeId();
    state.videoLayers.forEach(layer => layer.pendingCapture = false);
    state.maskLayers.forEach(layer => layer.pendingCapture = false);
}
export function updateOutputNodeId() {
    if (state.outputEntryId === null) {
        outputNodeId = null;
        return;
    }
    outputNodeId = cachedWiredIds.get(state.outputEntryId) ?? null;
}
export function getOutputNodeId() {
    return outputNodeId;
}

export function currentPreviewContentId() {
    const entry = scopedEntry(lastPreviewScope);
    if (!entry.id) return null;
    return cachedWiredIds.get(entry.id) ?? cachedContentIds.get(entry.id);
}
