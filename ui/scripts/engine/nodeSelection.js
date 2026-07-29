// nodeSelection.js
// Node selection state for the NODES menu context

/**
 * NodeSelectionState manages the currently selected node and preview channel
 * This is UI state only - it does not modify the graph or create operations
 */
export class NodeSelectionState {
    constructor() {
        this.selectedNode = null;  // Currently selected node entry (from registry)
        this.selectedChannel = "RGBA";  // Preview channel: RGBA, RED, GREEN, BLUE, ALPHA
        this.availableChannels = ["RGBA", "RED", "GREEN", "BLUE", "ALPHA"];
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
     * Set the preview channel
     * @param {string} channel - One of: RGBA, RED, GREEN, BLUE, ALPHA
     */
    setSelectedChannel(channel) {
        if (this.availableChannels.includes(channel)) {
            this.selectedChannel = channel;
        }
    }

    /**
     * Get the currently selected channel
     * @returns {string} The selected channel
     */
    getSelectedChannel() {
        return this.selectedChannel;
    }

    /**
     * Get the index of the currently selected channel
     * @returns {number} Index in availableChannels array
     */
    getSelectedChannelIndex() {
        return this.availableChannels.indexOf(this.selectedChannel);
    }

    /**
     * Select the next channel in the list (wraps around)
     */
    selectNextChannel() {
        const currentIndex = this.getSelectedChannelIndex();
        const nextIndex = (currentIndex + 1) % this.availableChannels.length;
        this.selectedChannel = this.availableChannels[nextIndex];
    }

    /**
     * Select the previous channel in the list (wraps around)
     */
    selectPreviousChannel() {
        const currentIndex = this.getSelectedChannelIndex();
        const prevIndex = (currentIndex - 1 + this.availableChannels.length) % this.availableChannels.length;
        this.selectedChannel = this.availableChannels[prevIndex];
    }

    /**
     * Check if the selected node supports editing
     * This will be extended as node-specific editors are implemented
     * @returns {boolean} True if the node supports editing
     */
    supportsEdit() {
        if (!this.selectedNode) return false;
        
        // For now, only certain node types support editing
        // This will be expanded as editors are implemented
        const editableNodeTypes = [
            "standaloneMask",  // Mask nodes will have threshold, feather, invert controls
            "composite",       // Composite nodes will have mode, opacity controls
            "rings",           // Rings will have animation/parameter controls
            "ghost"            // Ghost will have speed, loop, parameters
        ];
        
        return editableNodeTypes.includes(this.selectedNode.kind);
    }

    /**
     * Get all available channels
     * @returns {string[]} Array of channel names
     */
    getAvailableChannels() {
        return [...this.availableChannels];
    }
}

// Singleton instance
export const nodeSelectionState = new NodeSelectionState();
