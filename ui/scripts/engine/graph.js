/*
==================================================
GRAPH REBUILD - Incremental Update System
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
import {
    nodeSelectionState
} from "./nodeSelection.js";

// Track WASM node IDs for each layer to enable incremental updates
const layerNodeIds = new Map(); // layerId -> wasmNodeId
const layerSettingsHash = new Map(); // layerId -> hash of settings for change detection

let outputNodeId = null;
let lastPreviewScope = "video";

// Helper to create a stable hash from settings for change detection
function hashSettings(settings) {
    if (!settings) return "";
    const str = JSON.stringify(settings);
    // Simple hash - good enough for detecting changes
    let hash = 0;
    for (let i = 0; i < str.length; i++) {
        const char = str.charCodeAt(i);
        hash = ((hash << 5) - hash) + char;
        hash = hash & hash; // Convert to 32bit integer
    }
    return hash.toString();
}

// Helper to get or create a node ID for a layer
function getLayerNodeId(layerId) {
    return layerNodeIds.get(layerId) || null;
}

// Helper to set node ID for a layer
function setLayerNodeId(layerId, nodeId) {
    layerNodeIds.set(layerId, nodeId);
}

// Helper to check if layer settings changed
function settingsChanged(layerId, settings) {
    const currentHash = layerSettingsHash.get(layerId);
    const newHash = hashSettings(settings);
    if (currentHash !== newHash) {
        layerSettingsHash.set(layerId, newHash);
        return true;
    }
    return false;
}

function buildVideoContent(layer) {
    // Video/camera nodes are already created (and their pixel source
    // attached) at the point the layer was added. Just surface the node ID.
    if (layer.videoNodeId === undefined) return null;
    return layer.videoNodeId;
}

function buildImageContent(layer) {
    const wasmApp = getWasmApp();
    if (!wasmApp || layer.imageNodeId === undefined) return null;
    // Image nodes are already created with their data set
    // Just return the node ID
    return layer.imageNodeId;
}

function buildShuffleContent(layer) {
    // Shuffle nodes are created (and wired to their source, if any) directly
    // in the WASM graph when the user adds them - just surface the node ID.
    if (layer.nodeId === undefined) return null;
    return layer.nodeId;
}

// Incremental node builder - only rebuilds what changed
function buildLayerNode(entry) {
    const layer = entry.layer;
    const layerId = entry.id;
    const wasmApp = getWasmApp();
    if (!wasmApp) return null;

    // Check if we already have a node for this layer
    const existingNodeId = getLayerNodeId(layerId);
    
    // Check if settings changed
    const changed = settingsChanged(layerId, layer.settings);
    
    // If we have an existing node and settings haven't changed, reuse it
    if (existingNodeId !== null && !changed) {
        return existingNodeId;
    }

    // Build new node based on type
    let newNodeId = null;
    
    if (entry.kind === "video") {
        newNodeId = buildVideoContent(layer);
    } else if (entry.kind === "image") {
        newNodeId = buildImageContent(layer);
    } else if (entry.kind === "shuffle") {
        newNodeId = buildShuffleContent(layer);
    }
    
    if (newNodeId !== null) {
        // Clean up old node if it exists
        if (existingNodeId !== null) {
            // Note: Rust/WASM doesn't have node removal yet, 
            // but the generation mechanism in graph.rs will handle
            // stale IDs correctly when removal is implemented
        }
        setLayerNodeId(layerId, newNodeId);
    }
    
    return newNodeId;
}

export function rebuildGraph() {
    const wasmApp = getWasmApp();
    if (!wasmApp) return;
    
    const all = getAllRealEntries();
    const contentIds = new Map();
    
    /*
    FIRST PASS:
    Build primitive generators - these can be incrementally updated
    */
    all.forEach(entry => {
        if (entry.kind === "video" || entry.kind === "image" || entry.kind === "shuffle") {
            const nodeId = buildLayerNode(entry);
            if (nodeId !== null) {
                contentIds.set(entry.id, nodeId);
            }
        }
    });

    // Clean up stale entries from layerNodeIds for removed layers
    const currentLayerIds = new Set(all.map(e => e.id));
    for (const [layerId] of layerNodeIds) {
        if (!currentLayerIds.has(layerId)) {
            layerNodeIds.delete(layerId);
            layerSettingsHash.delete(layerId);
        }
    }

    // Update cached IDs for preview and output
    cachedContentIds = contentIds;
    cachedWiredIds = contentIds;

    updateOutputNodeId();
    state.videoLayers.forEach(layer => layer.pendingCapture = false);
}

// Legacy exports for compatibility
export let cachedContentIds = new Map();
export let cachedWiredIds = new Map();

export function updateOutputNodeId() {
    if (state.outputEntryId === null) {
        outputNodeId = null;
        return;
    }
    outputNodeId = cachedWiredIds.get(state.outputEntryId) ?? layerNodeIds.get(state.outputEntryId) ?? null;
}

export function getOutputNodeId() {
    return outputNodeId;
}

// Resolve the current WASM node ID backing a registry entry, wherever it
// sits in the wiring pipeline - used to connect one node's input to another.
export function resolveNodeId(entryId) {
    if (!entryId) return null;
    return cachedWiredIds.get(entryId) ?? cachedContentIds.get(entryId) ?? layerNodeIds.get(entryId) ?? null;
}

export function currentPreviewContentId() {
    // Check if there's an active node selection from the NODES menu
    const selectedNode = nodeSelectionState.getSelectedNode();
    if (selectedNode && selectedNode.id) {
        // Use the selected node's ID
        return cachedWiredIds.get(selectedNode.id) ?? cachedContentIds.get(selectedNode.id) ?? layerNodeIds.get(selectedNode.id);
    }
    
    // Fall back to scope-based preview (for legacy compatibility)
    const entry = scopedEntry(lastPreviewScope);
    if (!entry.id) return null;
    return cachedWiredIds.get(entry.id) ?? cachedContentIds.get(entry.id) ?? layerNodeIds.get(entry.id);
}
