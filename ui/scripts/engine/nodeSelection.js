// nodeSelection.js
// Node selection state for the NODES menu context

import { getWasmApp } from "../core/wasm.js";

/**
 * NodeSelectionState manages the currently selected node
 * This is UI state only - it does not modify the graph or create operations
 * 
 * Node selection contains only:
 * - selected node id
 * - selected node reference
 * - node navigation state
 * 
 * NO channel state - channel select will be a real Rust operation later
 */
export class NodeSelectionState {
    constructor() {
        this.selectedNode = null;  // Currently selected node entry (from registry)
    }

    /**
     * Set the selected node
     * @param {Object} nodeEntry - Node entry from registry (has id, label, kind, layer)
     */
    setSelectedNode(nodeEntry) {
        this.selectedNode = nodeEntry;
        // Dispatch event to notify that node selection changed
        window.dispatchEvent(new CustomEvent("nodeSelectionChanged", {
            detail: {
                node: nodeEntry
            }
        }));
    }

    /**
     * Get the currently selected node
     * @returns {Object|null} The selected node entry or null
     */
    getSelectedNode() {
        return this.selectedNode;
    }

    /**
     * Get the selected node's name/label
     * @returns {string} Node name or "NONE"
     */
    getSelectedNodeName() {
        return this.selectedNode ? this.selectedNode.label : "NONE";
    }

    /**
     * Check if the selected node supports editing
     * This queries the Rust WASM app to check if the node's operation supports edit.
     * @returns {boolean} True if the node supports editing
     */
    supportsEdit() {
        if (!this.selectedNode || !this.selectedNode.layer || this.selectedNode.layer.nodeId === undefined) {
            return false;
        }
        
        const wasmApp = getWasmApp();
        if (!wasmApp) {
            return false;
        }
        
        // Query Rust to check if this node supports editing
        return wasmApp.node_supports_edit(this.selectedNode.layer.nodeId);
    }
}

// Singleton instance
export const nodeSelectionState = new NodeSelectionState();
