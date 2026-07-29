// engine/menu.js

/** 
==================================================
DANCE TRACKER 5000
AMIGA TWO ROW MENU SYSTEM
==================================================
*/

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

    show(category) {
        console.log("Showing category:", category);
        this.category = category;
        this.render();
    }

    renderUpButton() {
        const upButton = document.createElement("button");
        upButton.innerText = "[UP]";
        upButton.className = "up-button";
        upButton.onclick = () => this.goUp();
        this.subMenu.appendChild(upButton);
    }

    render() {
        console.log("Rendering menu. Category:", this.category, "Operations count:", this.operations.length);

        this.subMenu.innerHTML = "";

        // Don't render if operations aren't loaded or no category selected
        if (this.operations.length === 0 || this.category === null) {
            console.log("Not rendering: no operations or no category");
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
                    this.renderButtons();
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
            this.render();
        }
    }
}
