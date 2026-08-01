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
        renderParameter(menuManager, startParamRow(menuManager), nodeId, parameter);
    });

    groupNames.forEach(groupName => {
        const row = startParamRow(menuManager);
        const groupButton = document.createElement("button");
        groupButton.innerText = `${groupName} >`;
        groupButton.onclick = () => menuManager.enterParameterGroup(groupName);
        row.appendChild(groupButton);
    });

    // PATCH has no real parameters() of its own (see operations::compose::patch) -
    // its entire interface is this mapping table. Dumping every property
    // row inline (the old behaviour) buried SOURCE/REFERENCE's own
    // steppers above under however many properties the wired SOURCE
    // happened to expose - a single drill-in button instead, same shape
    // as a parameter group's "<NAME> >" button.
    if (nodeEntry.kind === "patch") {
        renderPatchPropertiesButton(menuManager, nodeId);
    }
}

/**
 * The "PATCH PROPERTIES >" button on PATCH's main edit screen - appears
 * only once both of its wires (SOURCE and REFERENCE) are actually set,
 * since there's nothing real to map before then. Drills into its own
 * sub-pane (renderPatchPropertiesPane) rather than rendering the mapping
 * rows here directly.
 */
function renderPatchPropertiesButton(menuManager, nodeId) {
    const wasmApp = getWasmApp();
    if (!wasmApp) return;

    const inputs = wasmApp.node_inputs(nodeId);
    const sourceWired = inputs.find(input => input.name === "SOURCE")?.source != null;
    const referenceWired = inputs.find(input => input.name === "REFERENCE")?.source != null;
    if (!sourceWired || !referenceWired) return;

    const properties = wasmApp.patch_available_properties(nodeId);
    if (properties.length === 0) return;

    const row = startParamRow(menuManager);
    const button = document.createElement("button");
    button.innerText = "PATCH PROPERTIES >";
    button.onclick = () => menuManager.enterPatchProperties();
    row.appendChild(button);
}

/**
 * PATCH's own property<->output mapping sub-pane, pushed by the
 * "PATCH PROPERTIES >" button above (UP returns to PATCH's main edit
 * screen). One "<PROPERTY> <-- ___" stepper per property its wired
 * SOURCE (target) actually offers (real Number/Color parameters, or a
 * raw R/G/B/A fallback - see patch_available_properties()), cycling
 * through NONE + whichever outputs its wired REFERENCE (animation
 * source) declares.
 */
export function renderPatchPropertiesPane(menuManager, nodeEntry) {
    const nodeId = nodeEntry.layer.nodeId;
    renderParameterLabel(startParamRow(menuManager), "PATCH PROPERTIES");
    renderPatchMappingRows(menuManager, nodeId);
}

/**
 * Renders nothing until both wires exist - a defensive re-check in case
 * REFERENCE got disconnected while this sub-pane was already open (the
 * button above already gates entry, but that check ran on the previous
 * screen).
 */
function renderPatchMappingRows(menuManager, nodeId) {
    const wasmApp = getWasmApp();
    if (!wasmApp) return;

    const inputs = wasmApp.node_inputs(nodeId);
    const referenceInput = inputs.find(input => input.name === "REFERENCE");
    if (!referenceInput || referenceInput.source == null) return;

    const properties = wasmApp.patch_available_properties(nodeId);
    if (properties.length === 0) return;

    const outputs = wasmApp.node_outputs(referenceInput.source);
    const outputOptions = ["NONE", ...outputs.map(output => output.name)];

    properties.forEach(property => {
        const currentIndex = wasmApp.patch_mapping(nodeId, property);
        const optionIndex = (currentIndex !== null && currentIndex !== undefined) ? currentIndex + 1 : 0;

        const row = startParamRow(menuManager);
        renderParameterLabel(row, property);
        renderStepperButtons(row, outputOptions[optionIndex], direction => {
            const nextIndex = (optionIndex + direction + outputOptions.length) % outputOptions.length;
            if (nextIndex === 0) {
                wasmApp.clear_patch_mapping(nodeId, property);
            } else {
                wasmApp.set_patch_mapping(nodeId, property, nextIndex - 1);
            }
            menuManager.render();
        });
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

    renderParameterLabel(startParamRow(menuManager), groupName);

    parameters.forEach(parameter => renderParameter(menuManager, startParamRow(menuManager), nodeId, parameter));
}

/**
 * A parameter/input's entire row - its label, its control, nothing else -
 * always starts a fresh line regardless of how many other parameters this
 * node has or how wide the window is. flex: 1 0 100% inside the sub-menu's
 * own flex-wrap row is what forces the line break: a row item that's
 * always exactly as wide as the whole line pushes everything after it
 * onto the next one.
 */
function startParamRow(menuManager) {
    const row = document.createElement("div");
    row.className = "param-row";
    menuManager.subMenu.appendChild(row);
    return row;
}

/**
 * Render one parameter's control, dispatched by its kind. Shared by the
 * top-level (ungrouped) view and a parameter group's own sub-pane.
 */
function renderParameter(menuManager, container, nodeId, parameter) {
    if (parameter.kind === "COLOR") {
        renderColorParameter(container, nodeId, parameter);
        return;
    }

    if (parameter.kind === "NUMBER") {
        renderNumberParameter(menuManager, container, nodeId, parameter);
        return;
    }

    if (parameter.kind === "BOOLEAN") {
        renderBooleanParameter(menuManager, container, nodeId, parameter);
        return;
    }

    if (!parameter.options || parameter.options.length === 0) return;

    renderParameterLabel(container, parameter.name);

    const options = parameter.options;
    const currentIndex = Math.max(0, options.indexOf(parameter.value));

    renderStepperButtons(container, parameter.value, direction => {
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
function renderColorParameter(container, nodeId, parameter) {
    renderParameterLabel(container, parameter.name);

    const input = document.createElement("input");
    input.type = "color";
    input.value = parameter.value;
    input.oninput = () => dispatchParameterUpdate(nodeId, parameter.name, input.value);
    container.appendChild(input);
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
function renderNumberParameter(menuManager, container, nodeId, parameter) {
    renderParameterLabel(container, parameter.name);

    const current = parseFloat(parameter.value) || 0;
    const step = parameter.step ?? 1;

    // Always format for display at the step's own fixed decimal precision -
    // regardless of how many digits the raw value from Rust happens to
    // have (a value stepped before this fix existed is still just as
    // "dirty" as one stepped just now, and this formats it the same way
    // either way).
    const displayValue = current.toFixed(stepDecimals(step));

    renderStepperButtons(container, displayValue, direction => {
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
function renderBooleanParameter(menuManager, container, nodeId, parameter) {
    renderParameterLabel(container, parameter.name);

    const current = parameter.value === "true";

    renderStepperButtons(container, current ? "TRUE" : "FALSE", () => {
        dispatchParameterUpdate(nodeId, parameter.name, String(!current));
        menuManager.render();
    });
}

function renderParameterLabel(container, text) {
    const label = document.createElement("span");
    // Parameter/input names are plain identifiers (KEY_COLOR, SCALE_X) -
    // shown with spaces instead of underscores, never the raw identifier.
    label.innerText = ` ${text.replace(/_/g, " ")} `;
    label.className = "node-selector-label";
    container.appendChild(label);
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
 * -1 or +1 and is responsible for dispatching the update. `wide` widens
 * the value field for content that's legitimately longer than a typical
 * parameter value - a wired node's own display name, not a short number
 * or enum choice.
 */
function renderStepperButtons(container, valueText, onStep, wide = false) {
    const minusButton = document.createElement("button");
    minusButton.innerText = "-";
    minusButton.onclick = () => onStep(-1);
    container.appendChild(minusButton);

    const valueLabel = document.createElement("span");
    valueLabel.innerText = ` ${valueText} `;
    valueLabel.className = wide ? "node-value-label node-value-label-wide" : "node-value-label";
    container.appendChild(valueLabel);

    const plusButton = document.createElement("button");
    plusButton.innerText = "+";
    plusButton.onclick = () => onStep(1);
    container.appendChild(plusButton);
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

        const row = startParamRow(menuManager);
        renderParameterLabel(row, input.name);

        renderStepperButtons(row, options[currentIndex], direction => {
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
// These map node kinds to their respective edit UIs. video/camera sources
// currently declare no parameters or inputs (supports_edit() is false for
// them), so they never reach an edit context at all - only kinds that
// actually have something to edit need a registration here, and the
// generic default below is enough for all of them today.
nodeEditContextRegistry.registerDefault(renderGenericEditContext);
