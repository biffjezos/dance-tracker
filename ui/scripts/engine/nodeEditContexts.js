// nodeEditContexts.js
// Node-specific edit context registry

import { getWasmApp } from "../core/wasm.js";
import { getAllRealEntries } from "../state/registry.js";
import { resolveNodeId } from "./graph.js";

/**
 * NodeEditContext provides the structure for node-specific edit interfaces
 * Each node type that supports editing can register its own edit context
 * 
 * The registry ONLY renders UI controls - it does NOT perform any processing.
 * It answers: "What UI should be shown when this node is edited?"
 */
export class NodeEditContext {
    /**
     * Create a new node edit context
     * @param {string} nodeType - The node type this context handles (e.g., "video", "shuffle")
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
 * 
 * The registry is a pure UI concern - it maps node types to their edit UIs.
 * It does NOT modify the graph or perform any operations.
 */
export class NodeEditContextRegistry {
    constructor() {
        this.contexts = new Map(); // nodeType -> NodeEditContext
        this.defaultContext = null;
    }

    /**
     * Register the fallback edit context used for any node kind that has
     * no bespoke registration. New operations get a working EDIT screen
     * for free as long as they're generic enough for the default renderer.
     * @param {Function} renderFunction - Function to render the edit controls
     */
    registerDefault(renderFunction) {
        this.defaultContext = new NodeEditContext(null, renderFunction);
    }

    /**
     * Register a new edit context for a node type
     * @param {string} nodeType - The node type (e.g., "video", "shuffle")
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

        const context = this.getContext(nodeEntry.kind) || this.defaultContext;
        if (context) {
            context.render(menuManager, nodeEntry);
            return true;
        }

        return false;
    }
}

// Singleton instance
export const nodeEditContextRegistry = new NodeEditContextRegistry();

// ============================================================================
// NODE-SPECIFIC EDIT CONTEXT RENDERERS
// These functions render UI controls for specific node types.
// They do NOT perform any image processing or graph operations.
// ============================================================================

/**
 * Render function for IMAGE SOURCE edit context
 * Shows: REPLACE IMAGE, TRANSFORM controls
 */
export function renderImageEditContext(menuManager, nodeEntry) {
    const imageLabel = document.createElement("span");
    imageLabel.innerText = " IMAGE SOURCE SETTINGS ";
    imageLabel.className = "node-selector-label";
    menuManager.subMenu.appendChild(imageLabel);
}

/**
 * Render function for video edit context
 * Shows: ANIMATION CONTROLS
 */
export function renderVideoEditContext(menuManager, nodeEntry) {
    const videoLabel = document.createElement("span");
    videoLabel.innerText = " VIDEO EDIT: ANIMATION CONTROLS ";
    videoLabel.className = "node-selector-label";
    menuManager.subMenu.appendChild(videoLabel);
}

/**
 * Default edit context, used for any node kind without its own bespoke
 * registration.
 *
 * Layout: <NAME>  [-] [option selector] [+]  [-] [option selector] [+] ...
 *
 * Every enum-valued parameter is a stepper cycling through whatever values
 * the operation itself reports via node_parameters() - the UI never
 * hardcodes an option list, it only renders what Rust describes. Shuffle
 * was simply the first operation this was written against.
 */
export function renderGenericEditContext(menuManager, nodeEntry) {
    const wasmApp = getWasmApp();
    if (!wasmApp) return;

    const nodeId = nodeEntry.layer.nodeId;
    const parameters = wasmApp.node_parameters(nodeId);

    // Show the node label
    const nodeLabel = document.createElement("span");
    nodeLabel.innerText = ` ${nodeEntry.label} `;
    nodeLabel.className = "node-selector-label";
    menuManager.subMenu.appendChild(nodeLabel);

    parameters.forEach(parameter => {
        if (!parameter.options || parameter.options.length === 0) return;

        const options = parameter.options;
        const currentIndex = Math.max(0, options.indexOf(parameter.value));

        const step = direction => {
            const nextIndex = (currentIndex + direction + options.length) % options.length;
            window.dispatchEvent(
                new CustomEvent("updateNodeParameter", {
                    detail: {
                        nodeId: nodeId,
                        parameter: parameter.name,
                        value: options[nextIndex]
                    }
                })
            );
            menuManager.render();
        };

        const separator = document.createElement("span");
        separator.innerText = " ";
        menuManager.subMenu.appendChild(separator);

        const minusButton = document.createElement("button");
        minusButton.innerText = "-";
        minusButton.onclick = () => step(-1);
        menuManager.subMenu.appendChild(minusButton);

        const valueLabel = document.createElement("span");
        valueLabel.innerText = ` ${parameter.value} `;
        valueLabel.className = "node-selector-label";
        menuManager.subMenu.appendChild(valueLabel);

        const plusButton = document.createElement("button");
        plusButton.innerText = "+";
        plusButton.onclick = () => step(1);
        menuManager.subMenu.appendChild(plusButton);
    });

    renderInputSteppers(menuManager, nodeEntry, nodeId);
}

/**
 * Render a stepper for each input a node declares (queried from the graph,
 * never hardcoded), letting the user wire it to any other real node or back
 * to NONE. Generic across node kinds - Shuffle is simply the first user.
 */
function renderInputSteppers(menuManager, nodeEntry, nodeId) {
    const wasmApp = getWasmApp();
    if (!wasmApp) return;

    const inputs = wasmApp.node_inputs(nodeId);
    const candidates = getAllRealEntries().filter(entry => entry.id !== nodeEntry.id);

    inputs.forEach(input => {
        const options = ["NONE", ...candidates.map(entry => entry.label)];

        const currentEntry = candidates.find(entry => {
            const candidateId = resolveNodeId(entry.id);
            return candidateId !== null && candidateId === input.source;
        });
        const currentIndex = currentEntry ? candidates.indexOf(currentEntry) + 1 : 0;

        const step = direction => {
            const nextIndex = (currentIndex + direction + options.length) % options.length;
            if (nextIndex === 0) {
                wasmApp.disconnect_node_input(nodeId, input.name);
            } else {
                const target = candidates[nextIndex - 1];
                const sourceId = resolveNodeId(target.id);
                if (sourceId !== null) {
                    wasmApp.connect_node_input(nodeId, input.name, sourceId);
                }
            }
            menuManager.render();
        };

        const label = document.createElement("span");
        label.innerText = ` ${input.name} `;
        label.className = "node-selector-label";
        menuManager.subMenu.appendChild(label);

        const minusButton = document.createElement("button");
        minusButton.innerText = "-";
        minusButton.onclick = () => step(-1);
        menuManager.subMenu.appendChild(minusButton);

        const valueLabel = document.createElement("span");
        valueLabel.innerText = ` ${options[currentIndex]} `;
        valueLabel.className = "node-selector-label";
        menuManager.subMenu.appendChild(valueLabel);

        const plusButton = document.createElement("button");
        plusButton.innerText = "+";
        plusButton.onclick = () => step(1);
        menuManager.subMenu.appendChild(plusButton);
    });
}

// Register all known edit contexts
// These map node kinds to their respective edit UIs
nodeEditContextRegistry.register("video", renderVideoEditContext);
nodeEditContextRegistry.registerDefault(renderGenericEditContext);
