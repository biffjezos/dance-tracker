/*
==================================================
CANVAS FOCUS

Which panel (preview or output) is the current target for canvas-scoped
keyboard shortcuts (Space play/stop). Click a panel to focus it; only one
is focused at a time, shown by the border colour switching from cyan to
magenta.
==================================================
*/

let focusedPanel = null; // "preview" | "output" | null

export function getFocusedPanel() {
    return focusedPanel;
}

export function initCanvasFocus() {
    const panels = [
        { id: "preview", el: document.querySelector(".camera-panel") },
        { id: "output", el: document.querySelector(".output-panel") }
    ].filter(p => p.el);

    panels.forEach(({ id, el }) => {
        el.addEventListener("click", () => {
            focusedPanel = id;
            panels.forEach(p => p.el.classList.toggle("focused", p.id === id));
        });
    });
}
