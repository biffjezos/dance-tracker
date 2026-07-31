/*
==================================================
STATUS BAR / PREVIEW PANEL TITLE
==================================================
*/
import {
    scopedEntry,
    resolveMaskSourceLabel
} from "../state/registry.js";
import {
    renderPreview
} from "./render.js";
import { nodeSelectionState } from "./nodeSelection.js";
// Prefixed so which canvas is which is still obvious when only one is on
// screen at a time (single-panel/expanded mode) - otherwise both panels
// just show whatever node happens to be selected, with nothing to tell
// them apart.
const PREVIEW_PANEL_PREFIX = "PREVIEW: ";

function updateLayerStatusDisplay(entry) {
    const panelTitle = document.getElementById("camera-panel-title");
    if (panelTitle) {
        panelTitle.innerText = PREVIEW_PANEL_PREFIX + (entry.label || "NONE");
    }
    const bar = document.querySelector(".statusbar");
    if (bar) {
        bar.children[2].innerText = "TYPE: " + (entry.kind ? entry.kind.toUpperCase() : "NONE");
    }
}

function updateNodeSelectionDisplay() {
    const selectedNode = nodeSelectionState.getSelectedNode();
    const panelTitle = document.getElementById("camera-panel-title");
    if (panelTitle) {
        panelTitle.innerText = PREVIEW_PANEL_PREFIX + (selectedNode ? selectedNode.label : "NONE");
    }
    
    // Also update status bar if needed
    const bar = document.querySelector(".statusbar");
    if (bar && selectedNode) {
        bar.children[2].innerText = "TYPE: " + (selectedNode.kind ? selectedNode.kind.toUpperCase() : "NONE");
    }
    
    // Trigger preview render
    renderPreview();
}

let lastPreviewScope = "video";
export function reportSelection(scope) {
    lastPreviewScope = scope;
    const entry = scopedEntry(scope);
    updateLayerStatusDisplay(entry);
    window.dispatchEvent(new CustomEvent("maskSettingsChanged", {
        detail: {
            scope,
            source: entry.layer.settings.maskedBy.source,
            sourceLabel: resolveMaskSourceLabel(entry.layer.settings.maskedBy.source),
            channel: entry.layer.settings.maskedBy.channel
        }
    }));
    window.dispatchEvent(new CustomEvent("layerSelectionChanged", {
        detail: {
            scope,
            id: entry.id,
            label: entry.label,
            kind: entry.kind
        }
    }));
    renderPreview();
}

// Listen for node selection changes
window.addEventListener("nodeSelectionChanged", (e) => {
    updateNodeSelectionDisplay();
});
