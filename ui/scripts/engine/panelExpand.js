/*
==================================================
PANEL EXPAND/COLLAPSE

Either canvas panel can expand to fill the whole content area; the other
becomes hidden. isPanelVisible() is what render.js consults to skip that
panel's tick entirely while it's hidden - not just hiding the result, but
not computing it either.

Below MOBILE_BREAKPOINT (matches Bootstrap's lg), the two panels never
show side by side at all - squeezed to half width each isn't usable on a
narrow screen. With no explicit choice made yet, one panel (preview)
fills the screen by default there. Whenever only one panel is visible
(that default, or either panel manually expanded at any width), its own
title bar shows a .panel-switch-inline button to flip to the other one -
the per-panel expand buttons still work too, they're just harder to
reach for than a dedicated switch when only one panel is even visible to
click on.
==================================================
*/

const MOBILE_BREAKPOINT = 992;

let expandedPanel = null; // "preview" | "output" | null - explicit user choice
let workspaceEl = null;

function isNarrowViewport() {
    return window.innerWidth < MOBILE_BREAKPOINT;
}

function effectivePanel() {
    if (expandedPanel !== null) return expandedPanel;
    return isNarrowViewport() ? "preview" : null;
}

export function isPanelVisible(panelId) {
    const panel = effectivePanel();
    return panel === null || panel === panelId;
}

export function initPanelExpand() {
    workspaceEl = document.querySelector(".workspace");
    if (!workspaceEl) return;

    document.querySelectorAll(".expand-button").forEach(button => {
        button.addEventListener("click", () => {
            const panelId = button.dataset.panel;
            expandedPanel = expandedPanel === panelId ? null : panelId;
            applyExpandState();
        });
    });

    // Each panel's inline switch is only ever visible while its own panel
    // is the one currently shown (see the CSS), so clicking it always
    // means "go to the other one".
    document.querySelectorAll("[data-toggle-panel]").forEach(button => {
        button.addEventListener("click", () => {
            expandedPanel = button.dataset.togglePanel === "preview" ? "output" : "preview";
            applyExpandState();
        });
    });

    window.addEventListener("resize", applyExpandState);

    applyExpandState();
}

function applyExpandState() {
    const panel = effectivePanel();

    workspaceEl.classList.toggle("expanded-preview", panel === "preview");
    workspaceEl.classList.toggle("expanded-output", panel === "output");

    document.querySelectorAll(".expand-button").forEach(button => {
        button.innerText = button.dataset.panel === panel ? "⤡" : "⤢";
    });
}
