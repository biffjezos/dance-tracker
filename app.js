/*
==================================================
DANCE TRACKER 5000
APPLICATION CORE (Rust/WASM backend)

The compositor itself (sources, masks, generators, compose/blend,
transport) now lives in core/ as a Rust node graph compiled to WASM -
this file's job is purely translating menu.js's UI events into calls
against that graph, same as it translated them into JS class mutations
before.

SCOPE NOTE: this pass covers the features the rewrite was asked to
reach parity on - load (camera/video file), key (chroma/difference),
mask (MASKED BY), generate (rings/ghost/text), background chaining,
transport, and record. Audio sync, ring CONSTELLATION, per-stroke ring
recolouring, the eyedropper, and OUTPUT SIZE actually resizing are not
wired up yet - clicking those buttons is currently a no-op. Visibility
is binary (on/off) rather than the old four-mode system, since ALPHA/
MASK WHITE existed to solve a fixed-draw-order problem this graph
model doesn't have.

Stacking order (when more than one thing is independently visible with
no explicit BACKGROUND relationship between them) is fixed creation
order, oldest first - there's no MOVE UP/DOWN control in this menu.
==================================================
*/

import init, { App as WasmApp } from "./core/pkg/dance_tracker_core.js";
import { Camera } from "./engine/camera.js";
import { MenuManager } from "./engine/menu.js";
import { Recorder } from "./engine/recorder.js";


const WIDTH = 320;
const HEIGHT = 240;


const settings = { video: { width: WIDTH, height: HEIGHT } };

const camera = new Camera(settings);

const menu = new MenuManager();

let wasmApp = null;


/*
==================================================
REGISTRIES

Every real node the user can select/edit, JS-side. .content is the
wasm NodeId representing this layer's OWN appearance (source/mask/
generator, before background or masked-by wiring) - rebuildGraph()
below rewires background/maskedBy fresh every time anything changes,
and stores the fully-wired id back onto the entry as .wired.
==================================================
*/

let nextVideoNumber = 1;
let nextMaskNumber = 1;
let nextRingsNumber = 1;
let nextGhostNumber = 1;
let nextTextNumber = 1;

let cameraActivated = false;
let cameraOn = false;

const videoLayers = [];
const maskLayers = [];
const ringsLayers = [];
const ghostLayers = [];
const textLayers = [];

let selectedVideoIndex = 0;
let selectedMaskIndex = 0;

let transportPlaying = false;


function defaultUniversalSettings(){
    return {
        enabled:true,
        maskedBy:{source:"none", channel:"alpha"},
        background:{source:"none", blendMode:"normal"}
    };
}


/*
Every kind, in one flat list, in creation order - used for stepper
navigation, MASKED BY/BACKGROUND source pickers, and the fixed stacking
order.
*/
function getAllRealEntries(){
    const list = [];

    videoLayers.forEach(layer=>list.push({
        id:"video:" + layer.id,
        label:layer.name,
        kind:"video",
        layer
    }));

    maskLayers.forEach(layer=>list.push({
        id:"mask:" + layer.id,
        label:layer.name,
        kind:"standaloneMask",
        layer
    }));

    ringsLayers.forEach(layer=>list.push({
        id:"rings:" + layer.id,
        label:layer.name,
        kind:"rings",
        layer
    }));

    ghostLayers.forEach(layer=>list.push({
        id:"ghost:" + layer.id,
        label:layer.name,
        kind:"ghost",
        layer
    }));

    textLayers.forEach(layer=>list.push({
        id:"text:" + layer.id,
        label:layer.name,
        kind:"text",
        layer
    }));

    return list;
}


function getVideoRegistry(){
    return getAllRealEntries().filter(entry=>
        entry.kind === "video" ||
        entry.kind === "rings" ||
        entry.kind === "ghost" ||
        entry.kind === "text" ||
        entry.kind === "standaloneMask"
    );
}


function getMaskRegistry(){
    return getAllRealEntries().filter(entry=>entry.kind === "standaloneMask");
}


const EMPTY_ENTRY = {
    label:null,
    kind:null,
    id:null,
    layer:{settings:defaultUniversalSettings()}
};


function selectedVideoEntry(){
    const registry = getVideoRegistry();
    return registry[selectedVideoIndex] || registry[0] || EMPTY_ENTRY;
}


function selectedMaskEntry(){
    const registry = getMaskRegistry();
    return registry[selectedMaskIndex] || registry[0] || EMPTY_ENTRY;
}


function scopedEntry(scope){
    return scope === "mask" ? selectedMaskEntry() : selectedVideoEntry();
}


function resolveMaskSourceLabel(sourceId){
    if(!sourceId || sourceId === "none") return "NONE";
    const match = getAllRealEntries().find(entry=>entry.id === sourceId);
    return match ? match.label : "NONE";
}


/*
==================================================
STATUS BAR / CAMERA PANEL TITLE
==================================================
*/

const CAMERA_PANEL_DEFAULT_TITLE = "CAMERA INPUT";


function updateLayerStatusDisplay(entry){
    const panelTitle = document.getElementById("camera-panel-title");
    if(panelTitle){
        panelTitle.innerText = entry.label || CAMERA_PANEL_DEFAULT_TITLE;
    }

    const bar = document.querySelector(".statusbar");
    bar.children[2].innerText = "VISIBILITY: " + (entry.layer.settings.enabled ? "ON" : "OFF");
    bar.children[3].innerText = "TYPE: " + (entry.kind ? entry.kind.toUpperCase() : "NONE");
}


let lastPreviewScope = "video";


function reportSelection(scope){
    lastPreviewScope = scope;
    const entry = scopedEntry(scope);

    updateLayerStatusDisplay(entry);

    window.dispatchEvent(new CustomEvent("maskSettingsChanged", {
        detail:{
            scope,
            source:entry.layer.settings.maskedBy.source,
            sourceLabel:resolveMaskSourceLabel(entry.layer.settings.maskedBy.source),
            channel:entry.layer.settings.maskedBy.channel
        }
    }));

    window.dispatchEvent(new CustomEvent("backgroundSettingsChanged", {
        detail:{
            scope,
            source:entry.layer.settings.background.source,
            sourceLabel:resolveMaskSourceLabel(entry.layer.settings.background.source),
            colour:{r:0, g:0, b:0},
            blendMode:entry.layer.settings.background.blendMode
        }
    }));

    window.dispatchEvent(new CustomEvent("layerSelectionChanged", {
        detail:{
            scope,
            label:entry.label,
            kind:entry.kind,
            visibilityMode:entry.layer.settings.enabled ? "on" : "off",
            keyColour:
                entry.kind === "standaloneMask"
                ? entry.layer.settings.keyColour
                : null,
            mode:
                entry.kind === "standaloneMask"
                ? entry.layer.settings.mode
                : null,
            sourceLabel:
                entry.kind === "standaloneMask"
                ? resolveMaskSourceLabel(entry.layer.settings.source)
                : null
        }
    }));

    renderPreview();
}


/*
==================================================
GRAPH REBUILD

Rebuilds every node fresh from current JS-side settings and rewires
BACKGROUND/MASKED BY, then figures out the fixed-order visible stack
for the master output. Old wasm-side nodes from the previous rebuild
are simply abandoned (the wasm Graph has no removal API) - harmless,
since only nodes reachable from this rebuild's output/preview ids ever
get evaluated; it does mean a long session accumulates unused nodes in
wasm memory, a known, accepted tradeoff for how much simpler this is
than incremental graph surgery.
==================================================
*/

let outputNodeId = null;


function buildVideoContent(layer){
    if(!layer.videoEl) return null;
    return wasmApp.add_video_source(layer.videoEl);
}


function buildMaskContent(layer, contentIds){
    const s = layer.settings;
    const sourceContentId = contentIds.get(s.source);

    if(sourceContentId === undefined || sourceContentId === null) return null;

    if(s.mode === "keying"){
        return wasmApp.add_chroma(
            sourceContentId,
            s.keyColour.r, s.keyColour.g, s.keyColour.b,
            s.threshold,
            s.fill === "video",
            s.colour.r, s.colour.g, s.colour.b
        );
    }

    const differenceId = wasmApp.add_difference(
        sourceContentId,
        s.threshold,
        s.fill === "video",
        s.colour.r, s.colour.g, s.colour.b
    );

    if(layer.pendingCapture){
        wasmApp.capture_background(differenceId);
    }

    layer.lastDifferenceNodeId = differenceId;

    return differenceId;
}


function buildRingsContent(layer){
    return wasmApp.add_rings(
        layer.settings.count,
        layer.settings.ringsPerGroup,
        layer.settings.spacing,
        layer.settings.size,
        layer.settings.width
    );
}


function buildGhostContent(layer, contentIds){
    const sourceContentId = contentIds.get(layer.settings.applyToMask);
    if(sourceContentId === undefined || sourceContentId === null) return null;
    return wasmApp.add_ghost(sourceContentId, layer.settings.count, layer.settings.alpha, layer.settings.delayTicks);
}


function buildTextContent(layer){
    if(!layer.settings.content.trim()) return null;
    return wasmApp.add_text(layer.settings.content, layer.settings.colour, layer.settings.size);
}


function rebuildGraph(){
    if(!wasmApp) return;

    const all = getAllRealEntries();

    // Pass 1: each entry's own content, keyed by its universal id.
    const contentIds = new Map();

    all.forEach(entry=>{
        let id = null;

        if(entry.kind === "video") id = buildVideoContent(entry.layer);
        else if(entry.kind === "rings") id = buildRingsContent(entry.layer);
        else if(entry.kind === "text") id = buildTextContent(entry.layer);
        // mask/ghost need other entries' content ids resolved first - see pass 2.

        if(id !== null && id !== undefined) contentIds.set(entry.id, id);
    });

    all.forEach(entry=>{
        if(entry.kind === "standaloneMask"){
            const id = buildMaskContent(entry.layer, contentIds);
            if(id !== null && id !== undefined) contentIds.set(entry.id, id);
        }
    });

    all.forEach(entry=>{
        if(entry.kind === "ghost"){
            const id = buildGhostContent(entry.layer, contentIds);
            if(id !== null && id !== undefined) contentIds.set(entry.id, id);
        }
    });

    // Pass 2: wire MASKED BY then BACKGROUND on top of each entry's own content.
    const wiredIds = new Map();
    const consumed = new Set();

    all.forEach(entry=>{
        const s = entry.layer.settings;
        if(s.maskedBy.source !== "none") consumed.add(s.maskedBy.source);
        if(s.background.source !== "none") consumed.add(s.background.source);
    });

    all.forEach(entry=>{
        let id = contentIds.get(entry.id);
        if(id === undefined) return;

        const s = entry.layer.settings;

        if(s.maskedBy.source !== "none"){
            const maskId = contentIds.get(s.maskedBy.source);
            if(maskId !== undefined) id = wasmApp.add_apply_mask(id, maskId, s.maskedBy.channel);
        }

        if(s.background.source !== "none"){
            const bgId = wiredIds.get(s.background.source) !== undefined
                ? wiredIds.get(s.background.source)
                : contentIds.get(s.background.source);
            if(bgId !== undefined) id = wasmApp.add_compose(id, bgId, s.background.blendMode);
        }

        wiredIds.set(entry.id, id);
    });

    // Pass 3: fixed-order stack of everything independently visible and
    // not consumed by another entry's BACKGROUND/MASKED BY.
    let stackId = null;

    all.forEach(entry=>{
        if(!entry.layer.settings.enabled) return;
        if(consumed.has(entry.id)) return;

        const id = wiredIds.get(entry.id);
        if(id === undefined) return;

        stackId = stackId === null ? id : wasmApp.add_compose(id, stackId, "over");
    });

    outputNodeId = stackId;

    videoLayers.forEach(layer=>{ layer.pendingCapture = false; });
    maskLayers.forEach(layer=>{ layer.pendingCapture = false; });
}


function currentPreviewContentId(){
    const entry = scopedEntry(lastPreviewScope);
    if(!entry.id) return null;

    // Re-resolve through the same ids rebuildGraph just computed by
    // rebuilding a second, tiny map is wasteful - instead just rebuild
    // and grab this entry's own content id fresh, independent of
    // whatever the master stack looks like.
    const all = getAllRealEntries();
    const contentIds = new Map();

    all.forEach(e=>{
        let id = null;
        if(e.kind === "video") id = buildVideoContent(e.layer);
        else if(e.kind === "rings") id = buildRingsContent(e.layer);
        else if(e.kind === "text") id = buildTextContent(e.layer);
        if(id !== null && id !== undefined) contentIds.set(e.id, id);
    });

    if(entry.kind === "standaloneMask") return buildMaskContent(entry.layer, contentIds);
    if(entry.kind === "ghost") return buildGhostContent(entry.layer, contentIds);

    return contentIds.get(entry.id);
}


/*
Failure here is routine, not exceptional - a difference mask with
nothing captured yet, or a video whose first frame hasn't decoded,
throws every tick until that resolves on its own. Skipping the tick
silently (leaving the canvas at its last good frame) is the correct
behaviour; logging it as a console error would just be permanent noise
for a transient, expected state.
*/
function renderPreview(){
    if(!wasmApp) return;
    const canvas = document.getElementById("camera-preview");
    const id = currentPreviewContentId();
    if(id === null || id === undefined) return;

    try {
        wasmApp.preview_tick(id, canvas);
    }
    catch(error){
        // expected transient failure - see comment above
    }
}


function loop(){
    if(wasmApp && outputNodeId !== null && outputNodeId !== undefined){
        try {
            wasmApp.render_tick(outputNodeId, document.getElementById("master-layer"));
        }
        catch(error){
            // expected transient failure - see comment above
        }
    }

    renderPreview();

    requestAnimationFrame(loop);
}


/*
==================================================
INPUT: CAMERA / VIDEO FILE
==================================================
*/

const originalVideoLayer = {
    id:"original",
    number:null,
    name:null,
    videoEl:camera.getVideo(),
    settings:defaultUniversalSettings()
};


window.addEventListener("toggleCamera", ()=>{
    cameraOn = !cameraOn;

    if(cameraOn){
        if(!cameraActivated){
            cameraActivated = true;
            originalVideoLayer.number = nextVideoNumber++;
            originalVideoLayer.name = "VIDEO " + originalVideoLayer.number;
            videoLayers.push(originalVideoLayer);
            rebuildGraph();
            reportSelection("video");
        }

        transportPlaying = true;
        camera.start();
    }
    else {
        camera.stop();
    }
});


window.addEventListener("loadVideoFile", e=>{
    const video = document.createElement("video");
    video.muted = true;
    video.loop = true;
    video.playsInline = true;
    video.style.display = "none";
    document.body.appendChild(video);

    video.src = URL.createObjectURL(e.detail.file);
    video.play();

    const layer = {
        id:"video-" + (nextVideoNumber),
        number:nextVideoNumber++,
        name:"VIDEO " + (nextVideoNumber - 1),
        videoEl:video,
        settings:defaultUniversalSettings()
    };

    videoLayers.push(layer);
    transportPlaying = true;
    rebuildGraph();
    reportSelection("video");
});


window.addEventListener("addVideoLayer", e=>{
    const video = document.createElement("video");
    video.muted = true;
    video.loop = true;
    video.playsInline = true;
    video.style.display = "none";
    document.body.appendChild(video);

    video.src = URL.createObjectURL(e.detail.file);
    video.play();

    const layer = {
        id:"video-" + (nextVideoNumber),
        number:nextVideoNumber++,
        name:"VIDEO " + (nextVideoNumber - 1),
        videoEl:video,
        settings:defaultUniversalSettings()
    };

    videoLayers.push(layer);
    rebuildGraph();
    reportSelection("video");
});


/*
==================================================
KEY: MASKS
==================================================
*/

function addMaskLayer(){
    const layer = {
        id:"mask-" + nextMaskNumber,
        number:nextMaskNumber++,
        name:"MASK " + (nextMaskNumber - 1),
        settings:Object.assign(defaultUniversalSettings(), {
            mode:"difference",
            threshold:100,
            keyColour:{r:0, g:255, b:0},
            fill:"solid",
            colour:{r:255, g:0, b:255},
            source:"none"
        })
    };

    maskLayers.push(layer);
    return layer;
}


window.addEventListener("addMaskLayer", ()=>{
    addMaskLayer();
    rebuildGraph();
    reportSelection("mask");
});


window.addEventListener("maskVideoSourceStep", e=>{
    const entry = selectedMaskEntry();
    if(entry.kind !== "standaloneMask") return;

    const ids = ["none", ...getVideoRegistry().filter(v=>v.kind === "video").map(v=>v.id)];
    let index = ids.indexOf(entry.layer.settings.source);
    if(index < 0) index = 0;
    index = Math.min(Math.max(index + e.detail.direction, 0), ids.length - 1);

    entry.layer.settings.source = ids[index];
    rebuildGraph();
    reportSelection("mask");
});


window.addEventListener("captureLayerBackground", ()=>{
    const entry = selectedMaskEntry();
    if(entry.kind !== "standaloneMask") return;
    entry.layer.pendingCapture = true;
    rebuildGraph();
});


window.addEventListener("thresholdUp", ()=>{
    const s = selectedMaskEntry().layer.settings;
    if(!s) return;
    s.threshold += 5;
    rebuildGraph();
});


window.addEventListener("thresholdDown", ()=>{
    const s = selectedMaskEntry().layer.settings;
    if(!s) return;
    s.threshold = Math.max(0, s.threshold - 5);
    rebuildGraph();
});


window.addEventListener("toggleMatteMode", ()=>{
    const s = selectedMaskEntry().layer.settings;
    if(!s) return;
    s.mode = s.mode === "difference" ? "keying" : "difference";
    rebuildGraph();
    reportSelection("mask");
});


window.addEventListener("toggleLayerFill", ()=>{
    const s = selectedMaskEntry().layer.settings;
    if(!s) return;
    s.fill = s.fill === "solid" ? "video" : "solid";
    rebuildGraph();
});


window.addEventListener("layerColour", e=>{
    const s = selectedMaskEntry().layer.settings;
    if(!s) return;
    s.colour = {r:e.detail.r, g:e.detail.g, b:e.detail.b};
    rebuildGraph();
});


window.addEventListener("bodyKeyColour", e=>{
    const s = selectedMaskEntry().layer.settings;
    if(!s) return;
    s.keyColour = {r:e.detail.r, g:e.detail.g, b:e.detail.b};
    rebuildGraph();
});


/*
==================================================
UNIVERSAL ROW: stepper / visibility / background / masked by
==================================================
*/

window.addEventListener("videoIndexStep", e=>{
    const count = getVideoRegistry().length;
    selectedVideoIndex = Math.min(Math.max(selectedVideoIndex + e.detail.direction, 0), count - 1);
    reportSelection("video");
});


window.addEventListener("maskIndexStep", e=>{
    const count = getMaskRegistry().length;
    selectedMaskIndex = Math.min(Math.max(selectedMaskIndex + e.detail.direction, 0), count - 1);
    reportSelection("mask");
});


window.addEventListener("cycleVisibilityMode", e=>{
    const entry = scopedEntry(e.detail.scope);
    if(!entry.layer.settings) return;
    entry.layer.settings.enabled = !entry.layer.settings.enabled;
    rebuildGraph();
    reportSelection(e.detail.scope);
});


window.addEventListener("maskSourceStep", e=>{
    const entry = scopedEntry(e.detail.scope);
    const target = entry.layer.settings.maskedBy;
    const ids = ["none", ...getAllRealEntries().filter(o=>o.id !== entry.id).map(o=>o.id)];
    let index = ids.indexOf(target.source);
    if(index < 0) index = 0;
    index = Math.min(Math.max(index + e.detail.direction, 0), ids.length - 1);
    target.source = ids[index];
    rebuildGraph();
    reportSelection(e.detail.scope);
});


window.addEventListener("maskChannelStep", e=>{
    const entry = scopedEntry(e.detail.scope);
    const target = entry.layer.settings.maskedBy;
    const channels = ["red", "green", "blue", "alpha"];
    let index = channels.indexOf(target.channel);
    if(index < 0) index = 0;
    index = Math.min(Math.max(index + e.detail.direction, 0), channels.length - 1);
    target.channel = channels[index];
    rebuildGraph();
    reportSelection(e.detail.scope);
});


window.addEventListener("backgroundSourceStep", e=>{
    const entry = scopedEntry(e.detail.scope);
    const target = entry.layer.settings.background;
    const ids = ["none", ...getAllRealEntries().filter(o=>o.id !== entry.id).map(o=>o.id)];
    let index = ids.indexOf(target.source);
    if(index < 0) index = 0;
    index = Math.min(Math.max(index + e.detail.direction, 0), ids.length - 1);
    target.source = ids[index];
    rebuildGraph();
    reportSelection(e.detail.scope);
});


window.addEventListener("backgroundBlendModeStep", e=>{
    const entry = scopedEntry(e.detail.scope);
    const target = entry.layer.settings.background;
    const modes = ["normal", "multiply", "screen", "overlay", "darken", "lighten", "color-dodge", "color-burn", "hard-light", "soft-light", "difference", "exclusion"];
    let index = modes.indexOf(target.blendMode);
    if(index < 0) index = 0;
    index = Math.min(Math.max(index + e.detail.direction, 0), modes.length - 1);
    target.blendMode = modes[index];
    rebuildGraph();
    reportSelection(e.detail.scope);
});


window.addEventListener("layerBackgroundColour", ()=>{
    // Flat colour backgrounds aren't wired to the wasm graph in this
    // pass - BACKGROUND only supports another node as its source.
});


/*
==================================================
GENERATE: RINGS / GHOST / TEXT
==================================================
*/

function addRingsLayer(){
    const layer = {
        id:"rings-" + nextRingsNumber,
        number:nextRingsNumber++,
        name:"RINGS " + (nextRingsNumber - 1),
        settings:Object.assign(defaultUniversalSettings(), {
            count:2, ringsPerGroup:8, spacing:14, size:20, width:6
        })
    };
    ringsLayers.push(layer);
    return layer;
}


window.addEventListener("addRingsLayer", ()=>{
    addRingsLayer();
    rebuildGraph();
    reportSelection("video");
});


window.addEventListener("ringCountUp", ()=>{
    const s = selectedVideoEntry().layer.settings;
    if(s.count !== undefined && s.count < 8) s.count++;
    rebuildGraph();
});

window.addEventListener("ringCountDown", ()=>{
    const s = selectedVideoEntry().layer.settings;
    if(s.count !== undefined && s.count > 1) s.count--;
    rebuildGraph();
});

window.addEventListener("ringSizeUp", ()=>{
    const s = selectedVideoEntry().layer.settings;
    if(s.size !== undefined) s.size += 5;
    rebuildGraph();
});

window.addEventListener("ringSizeDown", ()=>{
    const s = selectedVideoEntry().layer.settings;
    if(s.size !== undefined) s.size = Math.max(5, s.size - 5);
    rebuildGraph();
});

window.addEventListener("ringThicknessUp", ()=>{
    const s = selectedVideoEntry().layer.settings;
    if(s.width !== undefined) s.width += 1;
    rebuildGraph();
});

window.addEventListener("ringThicknessDown", ()=>{
    const s = selectedVideoEntry().layer.settings;
    if(s.width !== undefined) s.width = Math.max(1, s.width - 1);
    rebuildGraph();
});


function addGhostLayer(){
    const layer = {
        id:"ghost-" + nextGhostNumber,
        number:nextGhostNumber++,
        name:"GHOST " + (nextGhostNumber - 1),
        settings:Object.assign(defaultUniversalSettings(), {
            count:3, alpha:0.45, delayTicks:3, applyToMask:"none"
        })
    };
    ghostLayers.push(layer);
    return layer;
}


window.addEventListener("addGhostLayer", ()=>{
    addGhostLayer();
    rebuildGraph();
    reportSelection("video");
});


window.addEventListener("ghostUp", ()=>{
    const s = selectedVideoEntry().layer.settings;
    if(s.count !== undefined) s.count++;
    rebuildGraph();
});

window.addEventListener("ghostDown", ()=>{
    const s = selectedVideoEntry().layer.settings;
    if(s.count !== undefined) s.count = Math.max(0, s.count - 1);
    rebuildGraph();
});

window.addEventListener("ghostDelayUp", ()=>{
    const s = selectedVideoEntry().layer.settings;
    if(s.delayTicks !== undefined) s.delayTicks++;
    rebuildGraph();
});

window.addEventListener("ghostDelayDown", ()=>{
    const s = selectedVideoEntry().layer.settings;
    if(s.delayTicks !== undefined) s.delayTicks = Math.max(1, s.delayTicks - 1);
    rebuildGraph();
});


function eligibleMaskTargets(excludeId){
    return getAllRealEntries().filter(entry=>entry.id !== excludeId);
}


window.addEventListener("requestApplyToMaskRefresh", e=>{
    const entry = selectedVideoEntry();
    if(entry.kind !== "ghost") return;

    const eligible = eligibleMaskTargets(entry.id);
    const match = eligible.find(target=>target.id === entry.layer.settings.applyToMask);

    e.detail.label = match ? match.label : "NONE AVAILABLE";
    e.detail.options = eligible.map(target=>({id:target.id, label:target.label}));
});


window.addEventListener("setApplyToMask", e=>{
    const entry = selectedVideoEntry();
    if(entry.kind !== "ghost") return;
    const eligible = eligibleMaskTargets(entry.id);
    const match = eligible.find(target=>target.id === e.detail.id);
    if(!match) return;
    entry.layer.settings.applyToMask = match.id;
    rebuildGraph();
});


function addTextLayer(){
    const layer = {
        id:"text-" + nextTextNumber,
        number:nextTextNumber++,
        name:"TEXT " + (nextTextNumber - 1),
        settings:Object.assign(defaultUniversalSettings(), {
            content:"", colour:"rgb(255,255,255)", size:24
        })
    };
    textLayers.push(layer);
    return layer;
}


window.addEventListener("addTextLayer", ()=>{
    addTextLayer();
    rebuildGraph();
    reportSelection("video");
});


window.addEventListener("setText", e=>{
    const s = selectedVideoEntry().layer.settings;
    if(s.content === undefined) return;
    s.content = e.detail.value;
    rebuildGraph();
});


window.addEventListener("textSizeUp", ()=>{
    const s = selectedVideoEntry().layer.settings;
    if(s.size !== undefined) s.size += 2;
    rebuildGraph();
});

window.addEventListener("textSizeDown", ()=>{
    const s = selectedVideoEntry().layer.settings;
    if(s.size !== undefined) s.size = Math.max(8, s.size - 2);
    rebuildGraph();
});


window.addEventListener("textColour", e=>{
    const s = selectedVideoEntry().layer.settings;
    if(s.colour === undefined) return;
    s.colour = "rgb(" + e.detail.r + "," + e.detail.g + "," + e.detail.b + ")";
    rebuildGraph();
});


/*
==================================================
TRANSPORT
==================================================
*/

function currentTransportVideo(){
    const entry = selectedVideoEntry();
    return (entry.layer && entry.layer.videoEl) || camera.getVideo();
}


function hasVideoFile(){
    const video = currentTransportVideo();
    return isFinite(video.duration) && video.duration > 0;
}


window.addEventListener("transportPlayStop", ()=>{
    if(!wasmApp) return;
    const video = currentTransportVideo();
    const hasFile = hasVideoFile();
    transportPlaying = hasFile ? video.paused : !transportPlaying;

    if(transportPlaying){
        if(hasFile) wasmApp.play(video);
    }
    else {
        if(hasFile) wasmApp.stop(video);
    }
});


function seekBy(seconds){
    if(!wasmApp) return;
    const video = currentTransportVideo();
    if(!hasVideoFile()) return;
    if(seconds >= 0) wasmApp.forward(video, seconds);
    else wasmApp.rewind(video, -seconds);
}


window.addEventListener("transportMinuteUp", ()=>seekBy(60));
window.addEventListener("transportMinuteDown", ()=>seekBy(-60));
window.addEventListener("transportSecondUp", ()=>seekBy(1));
window.addEventListener("transportSecondDown", ()=>seekBy(-1));
window.addEventListener("transportFrameUp", ()=>seekBy(1 / 30));
window.addEventListener("transportFrameDown", ()=>seekBy(-1 / 30));


/*
==================================================
OUTPUT / RECORDING
==================================================
*/

let recorder = null;


window.addEventListener("toggleRecord", ()=>{
    if(!recorder) recorder = new Recorder(document.getElementById("master-layer"));

    const bar = document.querySelector(".statusbar");

    if(recorder.recording){
        recorder.stop();
        bar.children[4].innerText = "REC: OFF";
    }
    else {
        recorder.start();
        bar.children[4].innerText = "REC: ON";
    }
});


/*
==================================================
NOT WIRED THIS PASS (documented, not silently missing)
==================================================
*/

window.addEventListener("outputSizeUp", ()=>{});
window.addEventListener("outputSizeDown", ()=>{});
window.addEventListener("toggleConstellation", ()=>{});
window.addEventListener("constellationDistanceUp", ()=>{});
window.addEventListener("constellationDistanceDown", ()=>{});
window.addEventListener("toggleRingsEnabled", ()=>{});
window.addEventListener("ringColour", ()=>{});
window.addEventListener("armKeyColourPicker", ()=>{});
window.addEventListener("audioSyncMinuteUp", ()=>{});
window.addEventListener("audioSyncMinuteDown", ()=>{});
window.addEventListener("audioSyncSecondUp", ()=>{});
window.addEventListener("audioSyncSecondDown", ()=>{});
window.addEventListener("audioSyncFrameUp", ()=>{});
window.addEventListener("audioSyncFrameDown", ()=>{});


/*
==================================================
BOOT
==================================================
*/

menu.init();

document.getElementById("master-layer").width = WIDTH;
document.getElementById("master-layer").height = HEIGHT;
document.getElementById("camera-preview").width = WIDTH;
document.getElementById("camera-preview").height = HEIGHT;


async function boot(){
    await init();
    wasmApp = new WasmApp(WIDTH, HEIGHT);

    reportSelection("video");
    requestAnimationFrame(loop);
}


boot();
