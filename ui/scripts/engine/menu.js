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

        this.show("input");
    }

    show(category) {
        this.category = category;
        this.render();
    }

    render() {
        this.subMenu.innerHTML = "";

        console.log("MENU OPS:", this.operations);

        this.operations.forEach(op => {
            const button = document.createElement("button");
            button.innerText = op.label || op.name || op;

            button.onclick = () => {
                window.dispatchEvent(
                    new CustomEvent("menuOperation", {
                        detail: op
                    })
                );
            };

            this.subMenu.appendChild(button);
        });
    }
}