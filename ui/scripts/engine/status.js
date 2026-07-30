/*
==================================================
STATUS BAR / CAMERA PANEL TITLE
==================================================
*/
import {
    scopedEntry,
    resolveMaskSourceLabel
} from "../state/registry.js";
import {
    state
} from "./state.js";
import {
    renderPreview
} from "./render.js";
import { nodeSelectionState } from "./nodeSelection.js";
const CAMERA_PANEL_DEFAULT_TITLE = "CAMERA INPUT";

function updateLayerStatusDisplay(entry) {
    const panelTitle = document.getElementById("camera-panel-title");
    if (panelTitle) {
        panelTitle.innerText = entry.label || CAMERA_PANEL_DEFAULT_TITLE;
    }
    const bar = document.querySelector(".statusbar");
    if (bar) {
        bar.children[2].innerText = "VISIBILITY: " + (entry.id !== null && entry.id === state.outputEntryId ? "ON" :
            "OFF");
        bar.children[3].innerText = "TYPE: " + (entry.kind ? entry.kind.toUpperCase() : "NONE");
    }
}

function updateNodeSelectionDisplay() {
    const selectedNode = nodeSelectionState.getSelectedNode();
    const panelTitle = document.getElementById("camera-panel-title");
    if (panelTitle) {
        panelTitle.innerText = selectedNode ? selectedNode.label : CAMERA_PANEL_DEFAULT_TITLE;
    }
    
    // Also update status bar if needed
    const bar = document.querySelector(".statusbar");
    if (bar && selectedNode) {
        bar.children[2].innerText = "VISIBILITY: " + (selectedNode.id !== null && selectedNode.id === state.outputEntryId ? "ON" : "OFF");
        bar.children[3].innerText = "TYPE: " + (selectedNode.kind ? selectedNode.kind.toUpperCase() : "NONE");
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
            kind: entry.kind,
            visibilityMode: entry.id !== null && entry.id === state.outputEntryId ? "on" : "off"
        }
    }));
    renderPreview();
}

// Listen for node selection changes
window.addEventListener("nodeSelectionChanged", (e) => {
    updateNodeSelectionDisplay();
});
