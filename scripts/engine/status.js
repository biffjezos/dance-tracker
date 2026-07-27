/*
==================================================
STATUS BAR / CAMERA PANEL TITLE
==================================================
*/
import {
    scopedEntry,
    resolveMaskSourceLabel
} from "./registry.js";
import {
    state
} from "./state.js";
import {
    renderPreview
} from "./render.js";
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
            visibilityMode: entry.id !== null && entry.id === state.outputEntryId ? "on" : "off",
            keyColour: entry.kind === "standaloneMask" ? entry.layer.settings.keyColour : null,
            mode: entry.kind === "standaloneMask" ? entry.layer.settings.mode : null,
            sourceLabel: entry.kind === "standaloneMask" ? resolveMaskSourceLabel(entry.layer.settings
                .source) : null,
            ringCount: entry.kind === "rings" ? entry.layer.settings.count : null,
            foregroundLabel: entry.kind === "composite" ? resolveMaskSourceLabel(entry.layer.settings
                .foreground) : null,
            backgroundLabel: entry.kind === "composite" ? resolveMaskSourceLabel(entry.layer.settings
                .background) : null,
            blendMode: entry.kind === "composite" ? entry.layer.settings.blendMode : null
        }
    }));
    renderPreview();
}