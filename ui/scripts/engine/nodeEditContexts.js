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

    // Wired inputs (SOURCE, FOREGROUND, BACKGROUND, ...) come first - what
    // this node is connected to is more fundamental than its own parameter
    // settings, and finding it shouldn't depend on scrolling past however
    // many parameters the operation happens to have. Renders nothing for a
    // node that declares no inputs (a source), so this is safe unconditionally.
    renderInputSteppers(menuManager, nodeEntry, nodeId);

    const parameters = wasmApp.node_parameters(nodeId);

    // Ungrouped parameters render inline, same as always. Grouped ones
    // collapse into one button per distinct group name - the operation
    // decides the grouping, this just renders whatever names it sees.
    const groupNames = [];
    parameters.forEach(parameter => {
        if (parameter.group) {
            if (!groupNames.includes(parameter.group)) groupNames.push(parameter.group);
            return;
        }
        renderParameter(menuManager, nodeId, parameter);
    });

    groupNames.forEach(groupName => {
        const groupButton = document.createElement("button");
        groupButton.innerText = `${groupName} >`;
        groupButton.onclick = () => menuManager.enterParameterGroup(groupName);
        menuManager.subMenu.appendChild(groupButton);
    });
}

/**
 * Render just the parameters belonging to one named group - the sub-pane
 * a group button (above) navigates into. Generic across node kinds; the
 * group name is entirely data from node_parameters(), never hardcoded.
 */
export function renderGroupContext(menuManager, nodeEntry, groupName) {
    const wasmApp = getWasmApp();
    if (!wasmApp) return;

    const nodeId = nodeEntry.layer.nodeId;
    const parameters = wasmApp.node_parameters(nodeId).filter(p => p.group === groupName);

    renderParameterLabel(menuManager, groupName);

    parameters.forEach(parameter => renderParameter(menuManager, nodeId, parameter));
}

/**
 * Render one parameter's control, dispatched by its kind. Shared by the
 * top-level (ungrouped) view and a parameter group's own sub-pane.
 */
function renderParameter(menuManager, nodeId, parameter) {
    if (parameter.kind === "COLOR") {
        renderColorParameter(menuManager, nodeId, parameter);
        return;
    }

    if (parameter.kind === "NUMBER") {
        renderNumberParameter(menuManager, nodeId, parameter);
        return;
    }

    if (parameter.kind === "BOOLEAN") {
        renderBooleanParameter(menuManager, nodeId, parameter);
        return;
    }

    if (!parameter.options || parameter.options.length === 0) return;

    renderParameterLabel(menuManager, parameter.name);

    const options = parameter.options;
    const currentIndex = Math.max(0, options.indexOf(parameter.value));

    renderStepperButtons(menuManager, parameter.value, direction => {
        const nextIndex = (currentIndex + direction + options.length) % options.length;
        dispatchParameterUpdate(nodeId, parameter.name, options[nextIndex]);
        menuManager.render();
    });
}

/**
 * Render a Color-kind parameter as a native colour picker. Its value is the
 * "#rrggbb" text Rust's value_to_text() already produces for Value::Color,
 * so the input's own value/change semantics are the entire implementation -
 * no palette, no bespoke widget.
 */
function renderColorParameter(menuManager, nodeId, parameter) {
    renderParameterLabel(menuManager, parameter.name);

    const input = document.createElement("input");
    input.type = "color";
    input.value = parameter.value;
    input.oninput = () => dispatchParameterUpdate(nodeId, parameter.name, input.value);
    menuManager.subMenu.appendChild(input);
}

/**
 * How many decimal places a step size implies (0.01 -> 2, 1 -> 0). The
 * single source of truth for both rounding a computed next-value and
 * formatting a value already on screen, so a NUMBER parameter's displayed
 * text is always the same fixed number of decimal digits - never 17 digits
 * of binary floating-point noise, and never a value with a different
 * decimal count than its neighbours.
 */
function stepDecimals(step) {
    return (String(step).split(".")[1] || "").length;
}

/**
 * Round to the step size's own decimal precision, so repeated fractional
 * steps (e.g. 0.3 + 0.01 several times over) don't accumulate binary
 * floating-point noise into values like 0.5700000000000002 in what gets
 * sent to Rust.
 */
function roundToStepPrecision(value, step) {
    const decimals = stepDecimals(step);
    if (decimals === 0) return Math.round(value);
    const factor = 10 ** decimals;
    return Math.round(value * factor) / factor;
}

/**
 * Render a Number-kind parameter as a stepper moving by the step size the
 * operation itself declares, clamped to its declared min/max so the
 * stepper can never send a value the operation would reject.
 */
function renderNumberParameter(menuManager, nodeId, parameter) {
    renderParameterLabel(menuManager, parameter.name);

    const current = parseFloat(parameter.value) || 0;
    const step = parameter.step ?? 1;

    // Always format for display at the step's own fixed decimal precision -
    // regardless of how many digits the raw value from Rust happens to
    // have (a value stepped before this fix existed is still just as
    // "dirty" as one stepped just now, and this formats it the same way
    // either way).
    const displayValue = current.toFixed(stepDecimals(step));

    renderStepperButtons(menuManager, displayValue, direction => {
        let next = roundToStepPrecision(current + direction * step, step);
        if (parameter.min != null) next = Math.max(parameter.min, next);
        if (parameter.max != null) next = Math.min(parameter.max, next);
        dispatchParameterUpdate(nodeId, parameter.name, String(next));
        menuManager.render();
    });
}

/**
 * Render a Boolean-kind parameter as a two-state stepper (TRUE/FALSE).
 */
function renderBooleanParameter(menuManager, nodeId, parameter) {
    renderParameterLabel(menuManager, parameter.name);

    const current = parameter.value === "true";

    renderStepperButtons(menuManager, current ? "TRUE" : "FALSE", () => {
        dispatchParameterUpdate(nodeId, parameter.name, String(!current));
        menuManager.render();
    });
}

function renderParameterLabel(menuManager, text) {
    const label = document.createElement("span");
    // Parameter/input names are plain identifiers (KEY_COLOR, SCALE_X) -
    // shown with spaces instead of underscores, never the raw identifier.
    label.innerText = ` ${text.replace(/_/g, " ")} `;
    label.className = "node-selector-label";
    menuManager.subMenu.appendChild(label);
}

function dispatchParameterUpdate(nodeId, parameterName, value) {
    window.dispatchEvent(
        new CustomEvent("updateNodeParameter", {
            detail: {
                nodeId: nodeId,
                parameter: parameterName,
                value: value
            }
        })
    );
}

/**
 * Render a "- value +" stepper triplet. onStep(direction) is called with
 * -1 or +1 and is responsible for dispatching the update.
 */
function renderStepperButtons(menuManager, valueText, onStep) {
    const separator = document.createElement("span");
    separator.innerText = " ";
    menuManager.subMenu.appendChild(separator);

    const minusButton = document.createElement("button");
    minusButton.innerText = "-";
    minusButton.onclick = () => onStep(-1);
    menuManager.subMenu.appendChild(minusButton);

    const valueLabel = document.createElement("span");
    valueLabel.innerText = ` ${valueText} `;
    valueLabel.className = "node-selector-label";
    menuManager.subMenu.appendChild(valueLabel);

    const plusButton = document.createElement("button");
    plusButton.innerText = "+";
    plusButton.onclick = () => onStep(1);
    menuManager.subMenu.appendChild(plusButton);
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

        renderParameterLabel(menuManager, input.name);

        renderStepperButtons(menuManager, options[currentIndex], direction => {
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
        });
    });
}

// Register all known edit contexts
// These map node kinds to their respective edit UIs
nodeEditContextRegistry.register("video", renderVideoEditContext);
nodeEditContextRegistry.registerDefault(renderGenericEditContext);
