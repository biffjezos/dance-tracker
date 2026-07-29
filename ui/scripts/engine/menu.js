// engine/menu.js

/** 
==================================================
DANCE TRACKER 5000
AMIGA TWO ROW MENU SYSTEM
==================================================
*/

import { nodeSelectionState } from "./nodeSelection.js";
import { nodeEditContextRegistry } from "./nodeEditContexts.js";
import { getAllRealEntries } from "../state/registry.js";

// MenuContext: Represents a menu in the hierarchy
class MenuContext {
    constructor(id, parent = null) {
        this.id = id;
        this.parent = parent;
    }
}

const rootMenu = new MenuContext("root");
const nodesMenu = new MenuContext("nodes", rootMenu);
const editMenu = new MenuContext("node_edit", nodesMenu);

const VISIBILITY_MODE_LABELS = {
    on: "ON",
    alpha: "ALPHA",
    maskWhite: "MASK WHITE",
    off: "OFF"
};
const SWATCHES = [{
    label: "BLACK",
    r: 0,
    g: 0,
    b: 0
}, {
    label: "WHITE",
    r: 255,
    g: 255,
    b: 255
}, {
    label: "RED",
    r: 255,
    g: 0,
    b: 0
}, {
    label: "GREEN",
    r: 0,
    g: 255,
    b: 0
}, {
    label: "BLUE",
    r: 0,
    g: 150,
    b: 255
}, {
    label: "MAGENTA",
    r: 255,
    g: 0,
    b: 255
}, {
    label: "CYAN",
    r: 0,
    g: 255,
    b: 255
}, {
    label: "YELLOW",
    r: 255,
    g: 255,
    b: 0
}, {
    label: "DARK GREEN",
    r: 0,
    g: 20,
    b: 0
}, {
    label: "DARK BLUE",
    r: 0,
    g: 10,
    b: 30
}];

function colourMenu(event) {
    return SWATCHES.map(swatch => ({
        label: swatch.label,
        event: event,
        r: swatch.r,
        g: swatch.g,
        b: swatch.b
    }));
}

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
        document.querySelectorAll(".main-menu button").forEach(button => {
            button.addEventListener("click", () => {
                console.log("Main menu clicked:", button.dataset.menu);
                this.show(button.dataset.menu);
            });
        });
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
        upButton.innerText = "[UP]";
        upButton.className = "up-button";
        upButton.onclick = () => this.goUp();
        this.subMenu.appendChild(upButton);
    }

    renderNodeSelector() {
        const selectionState = nodeSelectionState;
        
        // Create the node selector button group
        const minusButton = document.createElement("button");
        minusButton.innerText = "[-]";
        minusButton.onclick = () => {
            this.selectPreviousNode();
        };
        this.subMenu.appendChild(minusButton);

        const nodeLabel = document.createElement("span");
        nodeLabel.innerText = ` ${selectionState.getSelectedNodeName()} `;
        this.subMenu.appendChild(nodeLabel);

        const plusButton = document.createElement("button");
        plusButton.innerText = "[+]";
        plusButton.onclick = () => {
            this.selectNextNode();
        };
        this.subMenu.appendChild(plusButton);
    }

    renderChannelSelector() {
        const selectionState = nodeSelectionState;
        
        // Create separator
        const separator = document.createElement("span");
        separator.innerText = " | ";
        this.subMenu.appendChild(separator);

        // Create the channel selector button group
        const minusButton = document.createElement("button");
        minusButton.innerText = "[-]";
        minusButton.onclick = () => {
            selectionState.selectPreviousChannel();
            this.render();
        };
        this.subMenu.appendChild(minusButton);

        const channelLabel = document.createElement("span");
        channelLabel.innerText = ` ${selectionState.getSelectedChannel()} `;
        this.subMenu.appendChild(channelLabel);

        const plusButton = document.createElement("button");
        plusButton.innerText = "[+]";
        plusButton.onclick = () => {
            selectionState.selectNextChannel();
            this.render();
        };
        this.subMenu.appendChild(plusButton);
    }

    renderEditButton() {
        const selectionState = nodeSelectionState;
        
        if (selectionState.supportsEdit()) {
            // Create separator
            const separator = document.createElement("span");
            separator.innerText = " | ";
            this.subMenu.appendChild(separator);

            const editButton = document.createElement("button");
            editButton.innerText = "EDIT";
            editButton.onclick = () => {
                this.currentContext = editMenu;
                this.render();
                this.updateStatusBar();
            };
            this.subMenu.appendChild(editButton);
        }
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

        // Special handling for NODES menu
        if (this.category === "nodes") {
            // Render UP button if currentContext has a parent
            if (this.currentContext && this.currentContext.parent !== null) {
                this.renderUpButton();
                const separator = document.createElement("span");
                separator.innerText = " | ";
                this.subMenu.appendChild(separator);
            }

            // Render node selector and channel selector
            this.renderNodeSelector();
            this.renderChannelSelector();
            
            // Render EDIT button if supported
            this.renderEditButton();
            
            return;
        }

        // Special handling for EDIT context
        if (this.currentContext && this.currentContext.id === "node_edit") {
            // Render UP button
            if (this.currentContext.parent !== null) {
                this.renderUpButton();
                const separator = document.createElement("span");
                separator.innerText = " | ";
                this.subMenu.appendChild(separator);
            }
            
            // Render node-specific edit context
            const selectedNode = nodeSelectionState.getSelectedNode();
            if (selectedNode) {
                nodeEditContextRegistry.renderContext(this, selectedNode);
            } else {
                const editLabel = document.createElement("span");
                editLabel.innerText = " NODE EDIT CONTEXT ";
                this.subMenu.appendChild(editLabel);
            }
            return;
        }

        // Render UP button if currentContext has a parent
        if (this.currentContext && this.currentContext.parent !== null) {
            this.renderUpButton();
            const separator = document.createElement("span");
            separator.innerText = " | ";
            this.subMenu.appendChild(separator);
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

                if (op.buttons && op.buttons.length) {
                    console.log("Showing submenu buttons");
                    this.buttons = op.buttons;
                    // Create a child MenuContext for the submenu
                    this.currentContext = new MenuContext(
                        op.id || op.label.toLowerCase().replace(/\s+/g, "_"),
                        this.currentContext
                    );
                    this.renderButtons();
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

    renderButtons() {
        console.log("Rendering buttons submenu");
        this.subMenu.innerHTML = "";

        // Render UP button if currentContext has a parent
        if (this.currentContext && this.currentContext.parent !== null) {
            this.renderUpButton();
            const separator = document.createElement("span");
            separator.innerText = " | ";
            this.subMenu.appendChild(separator);
        }

        this.buttons.forEach(btn => {
            const button = document.createElement("button");
            button.innerText = btn.label || btn.name || btn;

            button.onclick = () => {
                console.log("Submenu button clicked, action:", btn.action);
                const action = btn.action;

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

    goUp() {
        if (this.currentContext && this.currentContext.parent) {
            this.currentContext = this.currentContext.parent;
            // If going up from nodes menu, reset category
            if (this.currentContext.id === "root") {
                this.category = null;
            } else {
                this.category = this.currentContext.id;
            }
            this.render();
            this.updateStatusBar();
        }
    }
}
