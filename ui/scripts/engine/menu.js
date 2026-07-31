// engine/menu.js

/** 
==================================================
DANCE TRACKER 5000
AMIGA TWO ROW MENU SYSTEM
==================================================
*/

import { nodeSelectionState } from "./nodeSelection.js";
import { nodeEditContextRegistry, renderGroupContext } from "./nodeEditContexts.js";
import { getAllRealEntries } from "../state/registry.js";
import { createNode } from "../core/operations.js";
import { getWasmApp } from "../core/wasm.js";
import {
    rebuildGraph,
    currentPreviewContentId,
    resolveNodeId
} from "./graph.js";
import { state, nextNumber } from "./state.js";

// MenuContext: Represents a menu in the hierarchy
class MenuContext {
    constructor(id, parent = null) {
        this.id = id;
        this.parent = parent;
        this.opId = null; // For create_node context, stores the operation ID
        this.group = null; // For param_group context, stores the group name
    }
}

const rootMenu = new MenuContext("root");
const nodesMenu = new MenuContext("nodes", rootMenu);

const VISIBILITY_MODE_LABELS = {
    on: "ON",
    alpha: "ALPHA",
    maskWhite: "MASK WHITE",
    off: "OFF"
};
export class MenuManager {
    constructor() {
        this.subMenu = document.getElementById("sub-menu");
        this.operations = [];
        this.category = null;
        this.currentContext = null;

        window.addEventListener("operationsLoaded", e => {
            console.log("Operations loaded:", e.detail);
            this.operations = e.detail;
        });
    }

    init() {
        // Only the actual category buttons - not the "☰ MENU" toggle
        // itself, which lives in .main-menu too but has no data-menu.
        document.querySelectorAll(".main-menu-items button").forEach(button => {
            button.addEventListener("click", () => {
                console.log("Main menu clicked:", button.dataset.menu);
                this.show(button.dataset.menu);
                this.closeMobileMenu();
            });
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
        console.log("Showing category:", category);
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
        const selectionState = nodeSelectionState;

        const minusButton = document.createElement("button");
        minusButton.innerText = "-";
        minusButton.disabled = disabled;
        if (!disabled) {
            minusButton.onclick = () => this.selectPreviousNode();
        }
        this.subMenu.appendChild(minusButton);

        const nodeLabel = document.createElement("span");
        nodeLabel.innerText = ` ${selectionState.getSelectedNodeName()} `;
        nodeLabel.className = "node-name-label";
        this.subMenu.appendChild(nodeLabel);

        const plusButton = document.createElement("button");
        plusButton.innerText = "+";
        plusButton.disabled = disabled;
        if (!disabled) {
            plusButton.onclick = () => this.selectNextNode();
        }
        this.subMenu.appendChild(plusButton);
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
                console.error("Failed to remove node:", err);
            }
        }

        state.videoLayers = state.videoLayers.filter(layer => layer !== selectedNode.layer);
        state.nodes = state.nodes.filter(layer => layer !== selectedNode.layer);

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
        console.log("Rendering menu. Category:", this.category, "Operations count:", this.operations.length);

        this.subMenu.innerHTML = "";

        // Don't render if operations aren't loaded or no category selected
        if (this.operations.length === 0 || this.category === null) {
            console.log("Not rendering: no operations or no category");
            return;
        }

        // Special handling for NODES menu - only at the top of the nodes
        // stack. Once EDIT (or anything else) has pushed a deeper context,
        // that context's own branch below must render instead.
        //
        // Every NODES-family context (this one, node_edit, param_group)
        // shares the same static part - [UP] [-] <node name> [+] - so
        // switching between them never jumps the layout. Only here is the
        // selector actually interactive; elsewhere it renders disabled.
        if (this.category === "nodes" && this.currentContext === nodesMenu) {
            if (this.currentContext.parent !== null) {
                this.renderUpButton();
            }
            this.renderNodeSelector();
            this.renderSeparator();

            this.renderEditButton();
            this.renderRemoveButton();

            // biffjezos: added [> live] button
            const liveButton = document.createElement("button");
            liveButton.innerText = "> LIVE PREVIEW";
            liveButton.onclick = () => {
                const nodeId = currentPreviewContentId();

                if (nodeId !== null && nodeId !== undefined) {
                    window.dispatchEvent(
                        new CustomEvent("setLiveNode", {
                            detail: {
                                nodeId,
                                label: nodeSelectionState.getSelectedNodeName()
                            }
                        })
                    );
                }
            };
            this.subMenu.appendChild(liveButton);
            return;
        }

        // Special handling for EDIT context.
        if (this.currentContext && this.currentContext.id === "node_edit") {
            if (this.currentContext.parent !== null) {
                this.renderUpButton();
            }
            this.renderNodeSelector(true);
            this.renderSeparator();

            const selectedNode = nodeSelectionState.getSelectedNode();
            if (selectedNode) {
                nodeEditContextRegistry.renderContext(this, selectedNode);
            }
            return;
        }

        // Special handling for a parameter-group sub-pane (pushed from
        // within an edit context when a parameter declares a group - e.g.
        // "COLOUR"). Generic across every node kind - node_parameters()
        // already tells us which parameters belong to the group.
        if (this.currentContext && this.currentContext.id === "param_group") {
            if (this.currentContext.parent !== null) {
                this.renderUpButton();
            }
            this.renderNodeSelector(true);
            this.renderSeparator();

            const selectedNode = nodeSelectionState.getSelectedNode();
            if (selectedNode) {
                renderGroupContext(this, selectedNode, this.currentContext.group);
            }
            return;
        }

        // Special handling for create_node context
        if (this.currentContext && this.currentContext.id === "create_node") {
            if (this.currentContext.parent !== null) {
                this.renderUpButton();
                this.renderSeparator();
            }


            // Render ADD button
            const addButton = document.createElement("button");
            addButton.innerText = "ADD";
            addButton.onclick = () => {
                // Get the operation ID from the context
                const opId = this.currentContext.opId;
                if (opId) {
                    this.createNodeAndSelect(opId);
                }
            };
            this.subMenu.appendChild(addButton);
            return;
        }

        // Render UP button if currentContext has a parent
        if (this.currentContext && this.currentContext.parent !== null) {
            this.renderUpButton();
            this.renderSeparator();
        }

        // Filter operations by the selected category (menu field)
        const filteredOps = this.operations.filter(op => {
            const opMenu = (op.menu || op.Menu || "").toUpperCase();
            const category = (this.category || "").toUpperCase();
            const match = opMenu === category;
            console.log(`Filtering op: menu="${op.menu || op.Menu}" vs category="${this.category}" -> ${match}`);
            return match;
        });

        console.log("Filtered operations:", filteredOps);

        filteredOps.forEach(op => {
            console.log("Creating button for:", op.label || op.name || op);
            const button = document.createElement("button");
            button.innerText = op.label || op.name || op;

            button.onclick = () => {
                console.log("Operation button clicked:", op);

                // Check if this operation creates a node
                if (op.create_node) {
                    console.log("Operation creates node:", op.create_node);
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
                console.log("Dispatching action:", action);

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
        console.log("Creating node:", operationId);

        // Create the node in the graph
        createNode(operationId).then(nodeId => {
            console.log("Node created with ID:", nodeId);

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
            state.nodes.push(layer);

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
            console.error("Failed to create node:", err);
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
