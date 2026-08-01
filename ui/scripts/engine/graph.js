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

function buildGenericContent(layer) {
    // Every layer's WASM node - video/camera, image, and any
    // create_node-backed operation alike - is already created (and its
    // pixel source or wiring attached, if it needs any) at the point the
    // layer was added. Just surface the node id.
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

    // buildGenericContent always just returns the WASM node id the layer
    // was created with (see its own comment) - there is no case here where
    // a *different* node id replaces an existing one, so there is nothing
    // to remove on the WASM side. wasmApp.remove_node() is what actually
    // deletes a node (see menu.js's removeSelectedNode), and is unrelated
    // to this rebuild path.
    const newNodeId = buildGenericContent(layer);

    if (newNodeId !== null) {
        setLayerNodeId(layerId, newNodeId);
    }

    return newNodeId;
}

export function rebuildGraph() {
    const wasmApp = getWasmApp();
    if (!wasmApp) return;
    
    const all = getAllRealEntries();
    const contentIds = new Map();
    
    // Build every real entry's WASM node. Every kind stores its id under
    // layer.nodeId, so a new kind works here with zero changes required.
    all.forEach(entry => {
        const nodeId = buildLayerNode(entry);
        if (nodeId !== null) {
            contentIds.set(entry.id, nodeId);
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

    // Cache the node ids this rebuild produced, for preview/output/wiring
    // lookups between rebuilds.
    cachedNodeIds = contentIds;
}

export let cachedNodeIds = new Map();

// Resolve the current WASM node ID backing a registry entry, wherever it
// sits in the wiring pipeline - used to connect one node's input to another.
export function resolveNodeId(entryId) {
    if (!entryId) return null;
    return cachedNodeIds.get(entryId) ?? layerNodeIds.get(entryId) ?? null;
}

// Walk upstream from a WASM node id to find the real HTMLVideoElement(s)
// feeding it - the node itself if it's a raw video/camera layer, or
// whatever video/camera layer(s) feed it through any number of intermediate
// nodes (blend modes, transforms, etc). A node can have more than one video
// feeding it (e.g. two inputs of a blend mode), so this returns all of them
// rather than assuming a single canonical source.
export function findVideoElementsForNode(nodeId, visited = new Set()) {
    if (nodeId === null || nodeId === undefined || visited.has(nodeId)) return [];
    visited.add(nodeId);

    const entry = getAllRealEntries().find(entry => resolveNodeId(entry.id) === nodeId);
    if (entry?.layer?.videoEl) {
        return [entry.layer.videoEl];
    }

    const wasmApp = getWasmApp();
    if (!wasmApp) return [];

    let inputs;
    try {
        inputs = wasmApp.node_inputs(nodeId);
    } catch (error) {
        return [];
    }

    const videoEls = [];
    for (const input of inputs) {
        if (input.source === null || input.source === undefined) continue;
        videoEls.push(...findVideoElementsForNode(input.source, visited));
    }
    return videoEls;
}

export function currentPreviewContentId() {
    // Check if there's an active node selection from the NODES menu
    const selectedNode = nodeSelectionState.getSelectedNode();
    if (selectedNode && selectedNode.id) {
        // Use the selected node's ID
        return cachedNodeIds.get(selectedNode.id) ?? layerNodeIds.get(selectedNode.id);
    }

    // Fall back to the single selected video/image/camera layer -
    // scopedEntry() only ever has one real scope ("video") today; see
    // registry.js's scopedEntry() for why the parameter exists at all.
    const entry = scopedEntry("video");
    if (!entry.id) return null;
    return cachedNodeIds.get(entry.id) ?? layerNodeIds.get(entry.id);
}
