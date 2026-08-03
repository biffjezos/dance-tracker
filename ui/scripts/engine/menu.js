// engine/menu.js

/** 
==================================================
DANCE TRACKER 5000
AMIGA TWO ROW MENU SYSTEM
==================================================
*/

import { nodeSelectionState } from "./nodeSelection.js";
import { nodeEditContextRegistry, renderGroupContext, renderPatchPropertiesPane } from "./nodeEditContexts.js";
import { getAllRealEntries, addNodeLayer, removeLayer } from "../state/registry.js";
import { createNode } from "../core/operations.js";
import { getWasmApp } from "../core/wasm.js";
import { logger } from "../core/log.js";
import {
    rebuildGraph,
    currentPreviewContentId,
    resolveNodeId
} from "./graph.js";
import { getLiveNodeId } from "./render.js";
import { nextNumber } from "./state.js";
import { systemMenus } from "./system_menu.js";
// MenuContext: Represents a menu in the hierarchy
class MenuContext {
    constructor(id, parent = null) {
        this.id = id;
        this.parent = parent;
        this.opId = null; // For create_node context, stores the operation ID
        this.group = null; // For param_group context, stores the group name
        this.submenu = null; // Within a menu's operation list, the active op.submenu name (if drilled into one)
    }
}

const rootMenu = new MenuContext("root");
const nodesMenu = new MenuContext("nodes", rootMenu);

// Purely cosmetic left-to-right preference for categories that already
// have a settled spot in the day-to-day workflow - it never gates
// whether a button appears (that's decided solely by what get_operations()
// actually reports, plus the always-present NODES browser). A category
// this list doesn't know about still gets a button, just appended
// alphabetically after the known ones.
const CATEGORY_ORDER = ["PROJECT", "INPUT", "NODES", "KEY", "GENERATE", "ANIMATE", "COMPOSE", "TRANSFORM", "OUTPUT"];

const VISIBILITY_MODE_LABELS = {
    on: "ON",
    alpha: "ALPHA",
    maskWhite: "MASK WHITE",
    off: "OFF"
};

// render() dispatches on MenuContext.id through this table instead of a
// growing if/else chain - adding a new special context (its own fixed
// layout, not a flat operation list) means adding one entry here, not
// another branch in render() itself. Contexts not listed here (the normal
// operation-category menus - INPUT, KEY, GENERATE, ...) fall through to
// the generic renderOperationList().
const CONTEXT_HANDLERS = {
    nodes: "renderNodesTopContext",
    node_edit: "renderNodeEditContext",
    param_group: "renderParamGroupContext",
    patch_properties: "renderPatchPropertiesContext",
    create_node: "renderCreateNodeContext",
};

// Only the states that mean something is actually broken get a tooltip -
// "missing_input" is the normal in-progress state of a node the user
// hasn't finished wiring yet, not an error to flag.
function describeNodeValidation(validation) {
    if (!validation) return null;
    switch (validation.state) {
        case "unknown_input":
            return `Wired to a removed node (#${validation.detail})`;
        case "invalid_dependency":
            return `Depends on a broken node (#${validation.detail})`;
        case "cycle":
            return "Part of a wiring cycle";
        default:
            return null;
    }
}
export class MenuManager {
    constructor() {
        this.subMenu = document.getElementById("sub-menu");
        this.operations = [];
        this.category = null;
        this.currentContext = null;

        window.addEventListener("operationsLoaded", e => {
            logger.debug("Operations loaded:", e.detail);
            this.operations = e.detail;
            this.renderCategoryButtons();
        });
    }

    // The main-menu category buttons are derived from whatever operations
    // are actually registered, not a hand-maintained list - a category
    // with nothing registered under it (yet) simply gets no button, per
    // "only display what actually exists". NODES is the one built-in
    // exception: it's always present since it's how you browse/edit nodes
    // that already exist, not tied to any operation's own menu field.
    renderCategoryButtons() {
        const items = document.getElementById("main-menu-items");
        if (!items) return;

        const categories = new Set(["NODES"]);

        systemMenus.forEach(menu => {
            categories.add(menu.menu);
        });
        this.operations.forEach(op => {
            const menu = (op.menu || op.Menu || "").toUpperCase();
            if (menu) categories.add(menu);
        });

        const known = CATEGORY_ORDER.filter(c => categories.has(c));
        const rest = [...categories].filter(c => !CATEGORY_ORDER.includes(c)).sort();

        items.innerHTML = "";
        [...known, ...rest].forEach(category => {
            const button = document.createElement("button");
            button.dataset.menu = category.toLowerCase();
            button.innerText = category;
            items.appendChild(button);
        });
    }

    init() {
        // Delegated on the container rather than bound per-button: category
        // buttons render dynamically once operations load (renderCategoryButtons),
        // so they don't exist yet at init() time. This naturally excludes the
        // "☰ MENU" toggle too, since it's a sibling of #main-menu-items, not
        // one of its children.
        const items = document.getElementById("main-menu-items");
        if (!items) return;
        items.addEventListener("click", e => {
            const button = e.target.closest("button[data-menu]");
            if (!button) return;
            this.show(button.dataset.menu);
            this.closeMobileMenu();
        });
    }

    /*
    Below lg, the category list is a floating dropdown (position:absolute)
    over the sub-menu it's about to populate - Bootstrap's Collapse never
    closes it on its own just because a button inside it was clicked, so
    without this the sub-menu renders correctly but stays completely
    hidden underneath the still-open dropdown. At lg+ this is a no-op:
    the dropdown markup is simply always shown there (d-lg-flex), so
    hiding the Collapse instance has nothing visible to do.
    */
    closeMobileMenu() {
        const items = document.getElementById("main-menu-items");
        if (!items || !window.bootstrap) return;
        window.bootstrap.Collapse.getOrCreateInstance(items, { toggle: false }).hide();
    }

    getContextPath() {
        const pathParts = [];
        let current = this.currentContext;
        while (current && current.id !== "root") {
            pathParts.push(current.id);
            current = current.parent;
        }
        return pathParts.reverse().join(" > ");
    }

    updateStatusBar() {
        const statusBar = document.querySelector(".statusbar span:first-child");
        if (statusBar) {
            const path = this.getContextPath();
            statusBar.textContent = `STATUS: ${path || "NO CONTEXT"}`;
        }
    }

    show(category) {
        this.category = category;
        // Set currentContext to the corresponding menu
        if (category === "nodes") {
            this.currentContext = nodesMenu;
            // Initialize node selection if not already set
            const selectionState = nodeSelectionState;
            if (!selectionState.getSelectedNode()) {
                const entries = getAllRealEntries();
                if (entries.length > 0) {
                    selectionState.setSelectedNode(entries[0]);
                }
            }
        } else {
            this.currentContext = new MenuContext(category, rootMenu);
        }
        this.render();
        this.updateStatusBar();
    }

    renderUpButton() {
        const upButton = document.createElement("button");
        upButton.innerText = "UP";
        upButton.className = "up-button";
        upButton.onclick = () => this.goUp();
        this.subMenu.appendChild(upButton);
    }

    renderSeparator() {
        const separator = document.createElement("span");
        separator.innerText = " | ";
        separator.className = "menu-separator";
        this.subMenu.appendChild(separator);
    }

    /**
     * The static part every NODES-family context shares:
     * [-] <fixed-width node name> [+]
     * Only the actual node selector (the "nodes" top context) can step
     * through nodes with it - everywhere else (EDIT, a parameter group) it
     * renders the same, disabled, so switching between these contexts
     * never jumps the layout around.
     */
    renderNodeSelector(disabled = false) {
        const minusButton = document.createElement("button");
        minusButton.innerText = "-";
        minusButton.disabled = disabled;
        if (!disabled) {
            minusButton.onclick = () => this.selectPreviousNode();
        }
        this.subMenu.appendChild(minusButton);

        this.renderNodeNameLabel();

        const plusButton = document.createElement("button");
        plusButton.innerText = "+";
        plusButton.disabled = disabled;
        if (!disabled) {
            plusButton.onclick = () => this.selectNextNode();
        }
        this.subMenu.appendChild(plusButton);
    }

    /**
     * The node name display itself, click-to-rename: click the text (not
     * the -/+ step buttons around it, which keep stepping through
     * different nodes exactly as before regardless of this) and it
     * becomes a real text input with a pink focus border; losing focus
     * commits whatever's typed as the node's new display name and reverts
     * to plain text. Renaming only ever changes what's displayed - the
     * WASM graph and the JS registry still only ever know this node by
     * its own immutable id, never by this label. The single shared
     * display used by NODES/EDIT/param-group alike, so renaming works
     * the same way everywhere the name shows up, not just from EDIT.
     */
    renderNodeNameLabel() {
        const selectedNode = nodeSelectionState.getSelectedNode();

        const nodeLabel = document.createElement("span");
        nodeLabel.innerText = ` ${nodeSelectionState.getSelectedNodeName()} `;
        nodeLabel.className = "node-name-label";

        if (selectedNode) {
            nodeLabel.classList.add("node-name-label-editable");
            nodeLabel.title = "Click to rename";
            nodeLabel.onclick = () => this.startRenamingNode(selectedNode, nodeLabel);

            this.applyValidationBadge(nodeLabel, selectedNode);
        }

        this.subMenu.appendChild(nodeLabel);
    }

    // Flags a node whose wiring is actually broken (a dangling reference, a
    // cycle, or depending on either), not merely not-yet-wired - an unwired
    // input on a freshly added node is completely normal per "no default
    // anything", not something to badge as an error.
    applyValidationBadge(nodeLabel, selectedNode) {
        const wasmApp = getWasmApp();
        const nodeId = resolveNodeId(selectedNode.id);
        if (!wasmApp || nodeId === null || nodeId === undefined) return;

        let validation;
        try {
            validation = wasmApp.node_validation(nodeId);
        } catch (error) {
            return;
        }

        const message = describeNodeValidation(validation);
        if (!message) return;

        nodeLabel.classList.add("node-name-label-invalid");
        nodeLabel.title = message;
    }

    startRenamingNode(selectedNode, nodeLabel) {
        const input = document.createElement("input");
        input.type = "text";
        input.className = "node-name-input";
        input.value = selectedNode.layer.name;

        const commit = () => {
            const trimmed = input.value.trim();
            if (trimmed) {
                selectedNode.layer.name = trimmed;
                // selectedNode is the exact object nodeSelectionState holds
                // as the current selection - its own .label is a snapshot
                // taken at selection time, not a live read of layer.name,
                // so it needs updating too or other displays of this same
                // selection (e.g. LIVE OUTPUT's title) would keep showing
                // the old name until something else reselects this node.
                selectedNode.label = trimmed;
            }
            this.render();
        };

        input.onblur = commit;
        input.onkeydown = e => {
            if (e.key === "Enter") input.blur();
            if (e.key === "Escape") {
                input.value = selectedNode.layer.name;
                input.blur();
            }
        };

        nodeLabel.replaceWith(input);
        input.focus();
        input.select();
    }

    renderEditButton() {
        const selectionState = nodeSelectionState;

        if (selectionState.supportsEdit()) {
            const editButton = document.createElement("button");
            editButton.innerText = "EDIT";
            editButton.onclick = () => {
                // Push the edit context directly on top of wherever EDIT was
                // clicked from - no intermediate pass-through context, so UP
                // goes straight back there in one step.
                this.currentContext = new MenuContext(
                    "node_edit",
                    this.currentContext
                );
                this.render();
                this.updateStatusBar();
            };
            this.subMenu.appendChild(editButton);
        }
    }

    renderRemoveButton() {
        if (!nodeSelectionState.getSelectedNode()) return;

        const removeButton = document.createElement("button");
        removeButton.innerText = "REMOVE";
        removeButton.onclick = () => this.removeSelectedNode();
        this.subMenu.appendChild(removeButton);
    }

    // Remove the selected node from the graph. Graph::remove_node already
    // safely disconnects anyone wired to it (no dangling reference, no
    // crash) - this just mirrors that on the JS side: drop the layer,
    // rebuild, and select whatever's left.
    removeSelectedNode() {
        const selectedNode = nodeSelectionState.getSelectedNode();
        if (!selectedNode) return;

        const wasmApp = getWasmApp();
        const nodeId = resolveNodeId(selectedNode.id);
        if (wasmApp && nodeId !== null && nodeId !== undefined) {
            try {
                wasmApp.remove_node(nodeId);
            } catch (err) {
                logger.error("Failed to remove node:", err);
            }
        }

        removeLayer(selectedNode.layer);

        rebuildGraph();

        const entries = getAllRealEntries();
        nodeSelectionState.setSelectedNode(entries.length > 0 ? entries[0] : null);

        this.render();
        this.updateStatusBar();
    }

    selectNextNode() {
        const selectionState = nodeSelectionState;
        const entries = getAllRealEntries();
        
        if (entries.length === 0) return;
        
        const currentNode = selectionState.getSelectedNode();
        let currentIndex = -1;
        
        // Find current index
        if (currentNode) {
            currentIndex = entries.findIndex(entry => entry.id === currentNode.id);
        }
        
        // Select next node (wrap around)
        const nextIndex = (currentIndex + 1) % entries.length;
        selectionState.setSelectedNode(entries[nextIndex]);
        
        this.render();
    }

    selectPreviousNode() {
        const selectionState = nodeSelectionState;
        const entries = getAllRealEntries();
        
        if (entries.length === 0) return;
        
        const currentNode = selectionState.getSelectedNode();
        let currentIndex = -1;
        
        // Find current index
        if (currentNode) {
            currentIndex = entries.findIndex(entry => entry.id === currentNode.id);
        }
        
        // Select previous node (wrap around)
        const prevIndex = (currentIndex - 1 + entries.length) % entries.length;
        selectionState.setSelectedNode(entries[prevIndex]);
        
        this.render();
    }

    render() {
        this.subMenu.innerHTML = "";

        // Don't render if operations aren't loaded or no category selected
        if (this.operations.length === 0 || this.category === null) {
            return;
        }

        // MenuContext.id is unique per kind of pushed context (the nodesMenu
        // singleton is the only context ever built with id "nodes"), so a
        // straight lookup replaces what used to be a chain of === checks.
        const handlerName = this.currentContext && CONTEXT_HANDLERS[this.currentContext.id];
        if (handlerName) {
            this[handlerName]();
            return;
        }

        this.renderOperationList();
    }

    // Every NODES-family context (top selector, EDIT, a parameter group)
    // shares this static part - [UP] [-] <node name> [+] - so switching
    // between them never jumps the layout. Only the top NODES context
    // (interactive=true) actually lets -/+ step through nodes; elsewhere
    // it renders disabled, same shape, so the layout still doesn't move.
    renderNodesFamilyHeader(interactive) {
        if (this.currentContext.parent !== null) {
            this.renderUpButton();
        }
        this.renderNodeSelector(!interactive);
        this.renderSeparator();
    }

    renderNodesTopContext() {
        this.renderNodesFamilyHeader(true);

        this.renderEditButton();
        this.renderRemoveButton();

        // biffjezos: added [> live] button
        // Toggle, not a one-way setter: clicking again on the node that's
        // already the live override releases it (dispatches clearLiveNode)
        // instead of leaving no way back to the wired LIVE OUTPUT node.
        const nodeId = currentPreviewContentId();
        const isLive = nodeId !== null && nodeId !== undefined && getLiveNodeId() === nodeId;
        const liveButton = document.createElement("button");
        liveButton.innerText = isLive ? "> RELEASE LIVE PREVIEW" : "> LIVE PREVIEW";
        liveButton.onclick = () => {
            if (isLive) {
                window.dispatchEvent(new CustomEvent("clearLiveNode"));
                this.render();
                return;
            }

            if (nodeId !== null && nodeId !== undefined) {
                window.dispatchEvent(
                    new CustomEvent("setLiveNode", {
                        detail: {
                            nodeId,
                            label: nodeSelectionState.getSelectedNodeName()
                        }
                    })
                );
                this.render();
            }
        };
        this.subMenu.appendChild(liveButton);
    }

    renderNodeEditContext() {
        this.renderNodesFamilyHeader(false);

        const selectedNode = nodeSelectionState.getSelectedNode();
        if (selectedNode) {
            nodeEditContextRegistry.renderContext(this, selectedNode);
        }
    }

    // A parameter-group sub-pane, pushed from within an edit context when a
    // parameter declares a group (e.g. "COLOUR"). Generic across every node
    // kind - node_parameters() already tells us which parameters belong to
    // the group.
    renderParamGroupContext() {
        this.renderNodesFamilyHeader(false);

        const selectedNode = nodeSelectionState.getSelectedNode();
        if (selectedNode) {
            renderGroupContext(this, selectedNode, this.currentContext.group);
        }
    }

    // PATCH's property-mapping sub-pane, pushed by its own "PATCH
    // PROPERTIES >" button - same push/pop shape as renderParamGroupContext
    // above, just PATCH-specific content instead of a named parameter group.
    renderPatchPropertiesContext() {
        this.renderNodesFamilyHeader(false);

        const selectedNode = nodeSelectionState.getSelectedNode();
        if (selectedNode) {
            renderPatchPropertiesPane(this, selectedNode);
        }
    }

    renderCreateNodeContext() {
        if (this.currentContext.parent !== null) {
            this.renderUpButton();
            this.renderSeparator();
        }

        const addButton = document.createElement("button");
        addButton.innerText = "ADD";
        addButton.onclick = () => {
            const opId = this.currentContext.opId;
            if (opId) {
                this.createNodeAndSelect(opId);
            }
        };
        this.subMenu.appendChild(addButton);
    }

    // The fallback for any context not in CONTEXT_HANDLERS - a normal
    // operation-category menu (INPUT, KEY, GENERATE, ...), rendered
    // entirely from get_operations() with no per-category code anywhere.
    //
    // Within that, an operation may declare a submenu (op.submenu, e.g.
    // "SPECTRA") for a second level of navigation under its menu - purely
    // presentational grouping the operation itself opts into, same idea as
    // a parameter's `group` in nodeEditContexts.js. Operations with no
    // submenu render as direct buttons regardless, so a menu with none of
    // its operations opted in renders exactly as before this existed.
    renderOperationList() {
        if (this.currentContext && this.currentContext.parent !== null) {
            this.renderUpButton();
            this.renderSeparator();
        }

        // Filter operations by the selected category (menu field)

        const menuEntries = [
            ...systemMenus,
            ...this.operations
        ];
        const filteredOps = menuEntries.filter(op => {
            const opMenu = (op.menu || op.Menu || "").toUpperCase();
            const category = (this.category || "").toUpperCase();
            return opMenu === category;
        });

        // A category button only renders when something is actually
        // registered under it (see renderCategoryButtons), so this is
        // currently unreachable in practice - kept as a defensive fallback
        // rather than silently leaving the sub-menu blank, per "if a
        // registry is empty, say so".
        if (filteredOps.length === 0) {
            const empty = document.createElement("span");
            empty.innerText = "NOTHING HERE YET";
            empty.className = "menu-empty";
            this.subMenu.appendChild(empty);
            return;
        }

        const activeSubmenu = this.currentContext && this.currentContext.submenu;

        // Already drilled into a submenu - show only its operations, no
        // further nesting.
        if (activeSubmenu) {
            this.renderOperationButtons(filteredOps.filter(op => op.submenu === activeSubmenu));
            return;
        }

        // Top level: operations with no submenu render directly; each
        // distinct submenu name among the rest collapses into one
        // "<NAME> >" button that drills in, exactly the same push/pop
        // pattern as enterParameterGroup()/param_group below.
        const submenuNames = [];
        filteredOps.forEach(op => {
            if (op.submenu && !submenuNames.includes(op.submenu)) {
                submenuNames.push(op.submenu);
            }
        });

        submenuNames.forEach(name => {
            const button = document.createElement("button");
            button.innerText = `${name} >`;
            button.onclick = () => {
                const submenuContext = new MenuContext(this.category, this.currentContext);
                submenuContext.submenu = name;
                this.currentContext = submenuContext;
                this.render();
                this.updateStatusBar();
            };
            this.subMenu.appendChild(button);
        });

        this.renderOperationButtons(filteredOps.filter(op => !op.submenu));
    }

    // One button per operation - triggers its create_node flow or
    // dispatches its ui_action/action. Shared by the top level of
    // renderOperationList() and its submenu drill-in.
    renderOperationButtons(ops) {
        ops.forEach(op => {
            const button = document.createElement("button");
            button.innerText = op.label || op.name || op;

            button.onclick = () => {
                logger.debug("Operation button clicked:", op);

                // Check if this operation creates a node
                if (op.create_node) {
                    // Create a child MenuContext for the create_node submenu
                    const createContext = new MenuContext("create_node", this.currentContext);
                    createContext.opId = op.create_node;
                    this.currentContext = createContext;
                    this.render();
                    this.updateStatusBar();
                    return;
                }

                // Use ui_action if available, otherwise fall back to action
                const action = op.ui_action || op.action;
                logger.debug("Dispatching action:", action);

                if (action) {
                    window.dispatchEvent(
                        new CustomEvent("menuOperation", {
                            detail: action
                        })
                    );
                }
            };
            this.subMenu.appendChild(button);
        });
    }

    createNodeAndSelect(operationId) {
        // Create the node in the graph
        createNode(operationId).then(nodeId => {
            logger.debug("Node created:", operationId, nodeId);

            // Every create_node-backed operation is stored the same way -
            // its display label comes from the operation's own descriptor,
            // never a hardcoded per-kind name.
            const op = this.operations.find(o => o.create_node === operationId);
            const label = op ? op.label : operationId.toUpperCase();
            const number = nextNumber(operationId);

            const layer = {
                id: `${operationId}-${number}`,
                name: `${label} ${number}`,
                nodeId: nodeId,
                kind: operationId,
                settings: {}
            };
            addNodeLayer(layer);

            // Make the new node's content available for preview/wiring
            rebuildGraph();

            // Select the new node - use the same registry-key convention
            // (kind + layer.id) as getAllRealEntries(), so this selection
            // matches what stepping through NODES will find for it later.
            const newEntry = {
                id: `${operationId}:${layer.id}`,
                label: layer.name,
                kind: operationId,
                layer
            };
            nodeSelectionState.setSelectedNode(newEntry);

            // Go back to nodes menu
            this.currentContext = nodesMenu;
            this.category = "nodes";

            // Refresh the menu
            this.render();
            this.updateStatusBar();
        }).catch(err => {
            logger.error("Failed to create node:", err);
        });
    }

    // Descend into a named parameter group's own sub-pane (UP returns here).
    enterParameterGroup(groupName) {
        const groupContext = new MenuContext("param_group", this.currentContext);
        groupContext.group = groupName;
        this.currentContext = groupContext;
        this.render();
        this.updateStatusBar();
    }

    // Descend into PATCH's own property-mapping sub-pane (UP returns here).
    enterPatchProperties() {
        const context = new MenuContext("patch_properties", this.currentContext);
        this.currentContext = context;
        this.render();
        this.updateStatusBar();
    }

    goUp() {
        if (this.currentContext && this.currentContext.parent) {
            this.currentContext = this.currentContext.parent;
            // If going up from nodes menu, reset category
            if (this.currentContext.id === "root") {
                this.category = null;
            } else if (this.currentContext.id === "nodes") {
                this.category = "nodes";
            } else {
                this.category = this.currentContext.id;
            }
            this.render();
            this.updateStatusBar();
        }
    }
}
