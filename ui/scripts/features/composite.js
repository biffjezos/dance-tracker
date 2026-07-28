/*
==================================================
COMPOSE: the only place two nodes get drawn together (see CLAUDE.md)
==================================================
*/
import {
    state
} from "../engine/state.js";
import {
    selectedCompositeEntry,
    getCompositeRegistry,
    getAllRealEntries,
    defaultUniversalSettings
} from "../state/registry.js";
import {
    rebuildGraph
} from "../engine/graph.js";
import {
    reportSelection
} from "../engine/status.js";

function stepSelection(registry, currentId, direction) {
    if (registry.length === 0) return null;
    let index = registry.findIndex(entry => entry.id === currentId);
    if (index < 0) index = 0;
    index += direction;
    if (index < 0) index = registry.length - 1;
    if (index >= registry.length) index = 0;
    return registry[index].id;
}

function addCompositeLayer() {
    const number = state.nextCompositeNumber++;
    const layer = {
        id: "composite-" + number,
        number,
        name: "COMPOSITE " + number,
        settings: {
            ...defaultUniversalSettings(),
            foreground: "none",
            background: "none",
            blendMode: "normal"
        }
    };
    state.compositeLayers.push(layer);
    return layer;
}
window.addEventListener("addCompositeLayer", () => {
    addCompositeLayer();
    rebuildGraph();
    reportSelection("composite");
});
window.addEventListener("compositeIndexStep", e => {
    state.selectedCompositeId = stepSelection(getCompositeRegistry(), state.selectedCompositeId, e.detail
        .direction);
    reportSelection("composite");
});
window.addEventListener("focusComposite", e => {
    if (!getCompositeRegistry().some(entry => entry.id === e.detail.id)) {
        return;
    }
    state.selectedCompositeId = e.detail.id;
    reportSelection("composite");
});
window.addEventListener("compositeForegroundStep", e => {
    const entry = selectedCompositeEntry();
    if (entry.kind !== "composite") return;
    const s = entry.layer.settings;
    const ids = ["none", ...getAllRealEntries().filter(o => o.id !== entry.id).map(o => o.id)];
    let index = ids.indexOf(s.foreground);
    if (index < 0) index = 0;
    index = Math.min(Math.max(index + e.detail.direction, 0), ids.length - 1);
    s.foreground = ids[index];
    rebuildGraph();
    reportSelection("composite");
});
window.addEventListener("compositeBackgroundStep", e => {
    const entry = selectedCompositeEntry();
    if (entry.kind !== "composite") return;
    const s = entry.layer.settings;
    const ids = ["none", ...getAllRealEntries().filter(o => o.id !== entry.id).map(o => o.id)];
    let index = ids.indexOf(s.background);
    if (index < 0) index = 0;
    index = Math.min(Math.max(index + e.detail.direction, 0), ids.length - 1);
    s.background = ids[index];
    rebuildGraph();
    reportSelection("composite");
});
window.addEventListener("compositeBlendModeStep", e => {
    const entry = selectedCompositeEntry();
    if (entry.kind !== "composite") return;
    const s = entry.layer.settings;
    const modes = ["normal", "multiply", "screen"];
    let index = modes.indexOf(s.blendMode);
    if (index < 0) index = 0;
    index = Math.min(Math.max(index + e.detail.direction, 0), modes.length - 1);
    s.blendMode = modes[index];
    rebuildGraph();
    reportSelection("composite");
});