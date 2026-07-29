// nodeSelection.js
// Node selection state for the NODES menu context

import { nodeEditContextRegistry } from "./nodeEditContexts.js";

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
     * This checks if the node has a registered edit context in the registry.
     * It means: "Does this node have an edit context?" NOT "Does this node modify pixels?"
     * @returns {boolean} True if the node has a registered edit context
     */
    supportsEdit() {
        if (!this.selectedNode) return false;
        
        // Check if the node's kind has a registered edit context
        return nodeEditContextRegistry.hasContext(this.selectedNode.kind);
    }
}

// Singleton instance
export const nodeSelectionState = new NodeSelectionState();
