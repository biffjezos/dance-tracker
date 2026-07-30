/*
==================================================
PANEL EXPAND/COLLAPSE

Either canvas panel can expand to fill the whole content area; the other
becomes hidden. isPanelVisible() is what render.js consults to skip that
panel's tick entirely while it's hidden - not just hiding the result, but
not computing it either.
==================================================
*/

let expandedPanel = null; // "preview" | "output" | null

export function isPanelVisible(panelId) {
    return expandedPanel === null || expandedPanel === panelId;
}

export function initPanelExpand() {
    const workspace = document.querySelector(".workspace");
    if (!workspace) return;

    document.querySelectorAll(".expand-button").forEach(button => {
        button.addEventListener("click", () => {
            const panelId = button.dataset.panel;
            expandedPanel = expandedPanel === panelId ? null : panelId;
            applyExpandState(workspace);
        });
    });
}

function applyExpandState(workspace) {
    workspace.classList.toggle("expanded-preview", expandedPanel === "preview");
    workspace.classList.toggle("expanded-output", expandedPanel === "output");

    document.querySelectorAll(".expand-button").forEach(button => {
        button.innerText = button.dataset.panel === expandedPanel ? "⤡" : "⤢";
    });
}
