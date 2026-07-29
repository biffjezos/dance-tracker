// engine/menu.js

/** 
==================================================
DANCE TRACKER 5000
AMIGA TWO ROW MENU SYSTEM
==================================================
*/
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
        this.category = "input";

        window.addEventListener("operationsLoaded", e => {
            this.operations = e.detail;
            this.render();
        });
    }

    init() {
        document.querySelectorAll(".main-menu button").forEach(button => {
            button.addEventListener("click", () => {
                this.show(button.dataset.menu);
            });
        });
    }

    show(category) {
        this.category = category;
        this.render();
    }

    render() {
        this.subMenu.innerHTML = "";

        // Don't render if operations aren't loaded yet
        if (this.operations.length === 0) {
            return;
        }

        // Filter operations by the selected category (menu field)
        const filteredOps = this.operations.filter(op => {
            const opMenu = (op.menu || op.Menu || "").toUpperCase();
            const category = (this.category || "").toUpperCase();
            return opMenu === category;
        });

        filteredOps.forEach(op => {
            const button = document.createElement("button");
            button.innerText = op.label || op.name || op;

            button.onclick = () => {
                if (op.buttons && op.buttons.length) {
                    this.buttons = op.buttons;
                    this.renderButtons();
                    return;
                }

                // Use ui_action if available, otherwise fall back to action
                const action = op.ui_action || op.action;

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
        this.subMenu.innerHTML = "";

        this.buttons.forEach(btn => {
            const button = document.createElement("button");
            button.innerText = btn.label || btn.name || btn;

            button.onclick = () => {
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
}
