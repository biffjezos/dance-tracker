// nodeEditContexts.js
// Node-specific edit context registry

/**
 * NodeEditContext provides the structure for node-specific edit interfaces
 * Each node type that supports editing can register its own edit context
 */
export class NodeEditContext {
    /**
     * Create a new node edit context
     * @param {string} nodeType - The node type this context handles (e.g., "standaloneMask", "composite")
     * @param {Function} renderFunction - Function that renders the edit controls for this node type
     */
    constructor(nodeType, renderFunction) {
        this.nodeType = nodeType;
        this.renderFunction = renderFunction;
    }

    /**
     * Render the edit controls for this node type
     * @param {MenuManager} menuManager - The menu manager to render with
     * @param {Object} nodeEntry - The selected node entry
     */
    render(menuManager, nodeEntry) {
        if (this.renderFunction) {
            this.renderFunction(menuManager, nodeEntry);
        }
    }
}

/**
 * NodeEditContextRegistry manages all available node edit contexts
 * This allows different node types to register their own edit interfaces
 */
export class NodeEditContextRegistry {
    constructor() {
        this.contexts = new Map(); // nodeType -> NodeEditContext
    }

    /**
     * Register a new edit context for a node type
     * @param {string} nodeType - The node type (e.g., "standaloneMask")
     * @param {Function} renderFunction - Function to render the edit controls
     */
    register(nodeType, renderFunction) {
        this.contexts.set(nodeType, new NodeEditContext(nodeType, renderFunction));
    }

    /**
     * Get the edit context for a specific node type
     * @param {string} nodeType - The node type to look up
     * @returns {NodeEditContext|null} The edit context or null if not found
     */
    getContext(nodeType) {
        return this.contexts.get(nodeType) || null;
    }

    /**
     * Check if a node type has a registered edit context
     * @param {string} nodeType - The node type to check
     * @returns {boolean} True if the node type has an edit context
     */
    hasContext(nodeType) {
        return this.contexts.has(nodeType);
    }

    /**
     * Render the appropriate edit context for a node entry
     * @param {MenuManager} menuManager - The menu manager to render with
     * @param {Object} nodeEntry - The selected node entry
     * @returns {boolean} True if an edit context was rendered, false otherwise
     */
    renderContext(menuManager, nodeEntry) {
        if (!nodeEntry || !nodeEntry.kind) return false;
        
        const context = this.getContext(nodeEntry.kind);
        if (context) {
            context.render(menuManager, nodeEntry);
            return true;
        }
        
        return false;
    }
}

// Singleton instance
export const nodeEditContextRegistry = new NodeEditContextRegistry();

// Placeholder render functions for future implementation
// These will be fleshed out as node-specific editors are implemented

/**
 * Placeholder render function for mask edit context
 */
export function renderMaskEditContext(menuManager, nodeEntry) {
    // Future implementation will include:
    // - THRESHOLD control
    // - FEATHER control  
    // - INVERT control
    const thresholdLabel = document.createElement("span");
    thresholdLabel.innerText = " MASK EDIT: THRESHOLD, FEATHER, INVERT ";
    thresholdLabel.className = "node-selector-label";
    menuManager.subMenu.appendChild(thresholdLabel);
}

/**
 * Placeholder render function for composite edit context
 */
export function renderCompositeEditContext(menuManager, nodeEntry) {
    // Future implementation will include:
    // - MODE control
    // - OPACITY control
    const compositeLabel = document.createElement("span");
    compositeLabel.innerText = " COMPOSITE EDIT: MODE, OPACITY ";
    compositeLabel.className = "node-selector-label";
    menuManager.subMenu.appendChild(compositeLabel);
}

/**
 * Placeholder render function for rings edit context
 */
export function renderRingsEditContext(menuManager, nodeEntry) {
    // Future implementation will include:
    // - Animation controls
    // - Parameter controls
    const ringsLabel = document.createElement("span");
    ringsLabel.innerText = " RINGS EDIT: ANIMATION, PARAMETERS ";
    ringsLabel.className = "node-selector-label";
    menuManager.subMenu.appendChild(ringsLabel);
}

/**
 * Placeholder render function for ghost edit context
 */
export function renderGhostEditContext(menuManager, nodeEntry) {
    // Future implementation will include:
    // - SPEED control
    // - LOOP control
    // - PARAMETERS control
    const ghostLabel = document.createElement("span");
    ghostLabel.innerText = " GHOST EDIT: SPEED, LOOP, PARAMETERS ";
    ghostLabel.className = "node-selector-label";
    menuManager.subMenu.appendChild(ghostLabel);
}

/**
 * Placeholder render function for image edit context
 */
export function renderImageEditContext(menuManager, nodeEntry) {
    // Future implementation will include:
    // - LOAD IMAGE
    // - RELOAD
    const imageLabel = document.createElement("span");
    imageLabel.innerText = " IMAGE EDIT: LOAD, RELOAD ";
    imageLabel.className = "node-selector-label";
    menuManager.subMenu.appendChild(imageLabel);
}

/**
 * Placeholder render function for video edit context
 */
export function renderVideoEditContext(menuManager, nodeEntry) {
    // Future implementation will include:
    // - Animation controls
    const videoLabel = document.createElement("span");
    videoLabel.innerText = " VIDEO EDIT: ANIMATION CONTROLS ";
    videoLabel.className = "node-selector-label";
    menuManager.subMenu.appendChild(videoLabel);
}

// Register all known edit contexts
nodeEditContextRegistry.register("standaloneMask", renderMaskEditContext);
nodeEditContextRegistry.register("composite", renderCompositeEditContext);
nodeEditContextRegistry.register("rings", renderRingsEditContext);
nodeEditContextRegistry.register("ghost", renderGhostEditContext);
nodeEditContextRegistry.register("image", renderImageEditContext);
nodeEditContextRegistry.register("video", renderVideoEditContext);
