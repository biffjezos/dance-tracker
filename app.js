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
import { containFit } from "./engine/fit.js";


const WIDTH = 320;
const HEIGHT = 240;

/*
Common, recognized, square-pixel resolutions only - no anamorphic
pixels, no letterboxed-on-4:3 legacy broadcast sizes (PALplus and
friends). Ordered by pixel count so OUTPUT SIZE +/- is a straight
"bigger/smaller" ladder across every aspect ratio family at once
(4:3, 1:1, 16:9) rather than three separate ladders.
*/
const OUTPUT_RESOLUTIONS = [
    { width:320, height:240 },   // QVGA, 4:3
    { width:640, height:480 },   // VGA, 4:3
    { width:800, height:600 },   // SVGA, 4:3
    { width:1024, height:768 },  // XGA, 4:3
    { width:1280, height:720 },  // HD, 16:9
    { width:1024, height:1024 }, // square
    { width:1600, height:1200 }, // UXGA, 4:3
    { width:1920, height:1080 }, // Full HD, 16:9
    { width:2560, height:1440 }, // QHD, 16:9
    { width:3840, height:2160 }  // 4K UHD, 16:9
];

let outputResolutionIndex = OUTPUT_RESOLUTIONS.findIndex(r => r.width === WIDTH && r.height === HEIGHT);
let outputWidth = WIDTH;
let outputHeight = HEIGHT;


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

// Which entry NODES/KEY's stepper is currently pointed at, by stable
// id - not a numeric index. getAllRealEntries() groups entries by
// kind, so adding something of an earlier kind (e.g. RINGS, which
// sorts before TEXT) shifts what a given *position* means; a numeric
// "index 1" could silently start pointing at a different node than
// the one the user was actually looking at. An id can't drift like
// that - it either still refers to the same real node, or (if that's
// somehow gone) selectedVideoEntry()/selectedMaskEntry() fall back to
// the first real entry same as an out-of-range index used to.
let selectedVideoId = null;
let selectedMaskId = null;

// Which single entry is actually live on the master output, by stable
// id - completely independent of the above. Merely stepping through
// NODES/KEY to look at something (or adding a new node elsewhere)
// must never change what's on air; only an explicit VISIBILITY click
// does that (see cycleVisibilityMode). Exclusive - turning one entry's
// visibility on takes it off whatever was previously live, since
// stacking only ever happens through an explicit BACKGROUND wire
// (see CLAUDE.md), never by having two things simultaneously "on".
let outputEntryId = null;

let transportPlaying = false;


function defaultUniversalSettings(){
    return {
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
    return registry.find(entry=>entry.id === selectedVideoId) || registry[0] || EMPTY_ENTRY;
}


function selectedMaskEntry(){
    const registry = getMaskRegistry();
    return registry.find(entry=>entry.id === selectedMaskId) || registry[0] || EMPTY_ENTRY;
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
    bar.children[2].innerText = "VISIBILITY: " + (entry.id !== null && entry.id === outputEntryId ? "ON" : "OFF");
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
            visibilityMode:entry.id !== null && entry.id === outputEntryId ? "on" : "off",
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
                : null,
            // How many ring groups/strokes this RINGS instance actually
            // has - RING EDIT's STROKE stepper must never offer more
            // than this many, see CLAUDE.md.
            ringCount:
                entry.kind === "rings"
                ? entry.layer.settings.count
                : null
        }
    }));

    renderPreview();
}


/*
==================================================
GRAPH REBUILD

Rebuilds every node fresh from current JS-side settings and rewires
BACKGROUND/MASKED BY, then points the master output at whichever entry
is currently selected. Old wasm-side nodes from the previous rebuild
are simply abandoned (the wasm Graph has no removal API) - harmless,
since only nodes reachable from this rebuild's output/preview ids ever
get evaluated; it does mean a long session accumulates unused nodes in
wasm memory, a known, accepted tradeoff for how much simpler this is
than incremental graph surgery.
==================================================
*/

let outputNodeId = null;

// Own-content ids from the most recent rebuildGraph() - reused by
// currentPreviewContentId() so every animation frame isn't minting
// fresh wasm nodes for whatever's on screen (see there for why that
// mattered).
let cachedContentIds = new Map();

// Fully-wired (masked-by + background applied) ids from the most
// recent rebuildGraph() - reused by updateOutputNodeId() so stepping
// to a different node updates the output without needing a full
// rebuild.
let cachedWiredIds = new Map();


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
        layer.settings.width,
        layer.settings.colours
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

    cachedContentIds = contentIds;
    cachedWiredIds = wiredIds;
    updateOutputNodeId();

    videoLayers.forEach(layer=>{ layer.pendingCapture = false; });
    maskLayers.forEach(layer=>{ layer.pendingCapture = false; });
}


/*
Output is exactly whichever single entry is currently marked live
(outputEntryId), rendered through its own fully-wired node (its own
MASKED BY / BACKGROUND, if the user set them) - never an automatic
merge with anything else. Stacking happens only through an explicit
BACKGROUND wire the user set themselves (see CLAUDE.md).

Deliberately NOT derived from "whatever NODES/KEY is currently
scrolled to" - outputEntryId only changes via an explicit VISIBILITY
click (cycleVisibilityMode). Merely stepping through the list to look
at or prepare something else, or adding a new node elsewhere, must
never disturb what's actually live - that's the whole point of
separating "what am I looking at" from "what's on air".

Called from rebuildGraph() (the live entry's own wiring may have
changed) and from cycleVisibilityMode (outputEntryId itself changed).
*/
function updateOutputNodeId(){
    if(outputEntryId === null){
        outputNodeId = null;
        return;
    }

    const currentId = cachedWiredIds.get(outputEntryId);
    outputNodeId = currentId !== undefined ? currentId : null;
}


/*
Reuses rebuildGraph()'s own-content ids instead of minting fresh wasm
nodes every animation frame - re-running add_rings/add_video_source/etc
every tick handed a stateful generator (Rings' wandering ring centres,
a difference mask's captured background) a brand new node with none of
its previous state every single frame, which is what made RINGS
visibly jump around here even though the master output (built once per
rebuildGraph(), not once per frame) looked fine.

A standalone mask previews its SOURCE, not its own keyed output - you
need to see the raw video to tell what you're keying (pick a colour,
frame the shot), not the result of a key you haven't tuned yet.
*/
function currentPreviewContentId(){
    const entry = scopedEntry(lastPreviewScope);
    if(!entry.id) return null;

    if(entry.kind === "standaloneMask") return cachedContentIds.get(entry.layer.settings.source);

    return cachedContentIds.get(entry.id);
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
    const masterCanvas = document.getElementById("master-layer");

    if(wasmApp && outputNodeId !== null && outputNodeId !== undefined){
        try {
            wasmApp.render_tick(outputNodeId, masterCanvas);
        }
        catch(error){
            // expected transient failure - see comment above
        }
    }
    else {
        // Nothing selected/enabled for output - go blank. Leaving the
        // canvas showing its last rendered frame instead would freeze
        // a live feed mid-frame, which reads as "the video stopped".
        masterCanvas.getContext("2d").clearRect(0, 0, masterCanvas.width, masterCanvas.height);
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


let armedKeyColourPick = false;


/*
Samples from #camera-preview, not any one fixed video - that canvas
already shows whatever's currently relevant (see
currentPreviewContentId, and note a mask's preview is its own SOURCE
video precisely so there's something meaningful to pick a colour
from), so picking a colour from what you're actually looking at is
both simpler and more correct than hardcoding one particular video.
*/
window.addEventListener("armKeyColourPicker", ()=>{
    armedKeyColourPick = true;
    document.getElementById("camera-preview").classList.add("sampling");
});


document.getElementById("camera-preview").addEventListener("click", e=>{
    if(!armedKeyColourPick) return;
    armedKeyColourPick = false;

    const source = e.target;
    source.classList.remove("sampling");

    const rect = source.getBoundingClientRect();
    const fit = containFit(source.width, source.height, rect.width, rect.height);

    const x = Math.floor((e.clientX - rect.left - fit.x) / fit.width * source.width);
    const y = Math.floor((e.clientY - rect.top - fit.y) / fit.height * source.height);

    if(x < 0 || y < 0 || x >= source.width || y >= source.height) return;

    const pixel = source.getContext("2d").getImageData(x, y, 1, 1).data;

    window.dispatchEvent(new CustomEvent("bodyKeyColour", {
        detail:{r:pixel[0], g:pixel[1], b:pixel[2]}
    }));
});


/*
==================================================
UNIVERSAL ROW: stepper / visibility / background / masked by
==================================================
*/

function stepSelection(registry, currentId, direction){
    const currentIndex = registry.findIndex(entry=>entry.id === currentId);
    const nextIndex = Math.min(Math.max((currentIndex < 0 ? 0 : currentIndex) + direction, 0), registry.length - 1);
    const next = registry[nextIndex];
    return next ? next.id : null;
}


window.addEventListener("videoIndexStep", e=>{
    selectedVideoId = stepSelection(getVideoRegistry(), selectedVideoId, e.detail.direction);
    reportSelection("video");
});


window.addEventListener("maskIndexStep", e=>{
    selectedMaskId = stepSelection(getMaskRegistry(), selectedMaskId, e.detail.direction);
    reportSelection("mask");
});


window.addEventListener("cycleVisibilityMode", e=>{
    const entry = scopedEntry(e.detail.scope);
    if(!entry.id) return;

    // Exclusive: making this entry live takes any other entry off air
    // - there is no implicit multi-thing stack, only an explicit
    // BACKGROUND wire composites two things together (see CLAUDE.md).
    outputEntryId = outputEntryId === entry.id ? null : entry.id;

    updateOutputNodeId();
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
            count:2, ringsPerGroup:8, spacing:14, size:20, width:6,
            colours:["rgb(255,0,255)", "rgb(0,255,80)"]
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


window.addEventListener("ringColour", e=>{
    const s = selectedVideoEntry().layer.settings;
    if(!Array.isArray(s.colours)) return;

    const index = e.detail.ringId - 1;
    while(s.colours.length <= index) s.colours.push("rgb(255,0,255)");
    s.colours[index] = "rgb(" + e.detail.r + "," + e.detail.g + "," + e.detail.b + ")";

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

function applyOutputSize(){
    if(!wasmApp) return;
    wasmApp.set_resolution(outputWidth, outputHeight);

    const aspectRatio = (outputWidth / outputHeight).toFixed(3);
    const bar = document.querySelector(".statusbar");
    bar.children[5].innerText = outputWidth + "x" + outputHeight + " " + aspectRatio;
}


window.addEventListener("outputSizeUp", ()=>{
    outputResolutionIndex = Math.min(OUTPUT_RESOLUTIONS.length - 1, outputResolutionIndex + 1);
    outputWidth = OUTPUT_RESOLUTIONS[outputResolutionIndex].width;
    outputHeight = OUTPUT_RESOLUTIONS[outputResolutionIndex].height;
    applyOutputSize();
});


window.addEventListener("outputSizeDown", ()=>{
    outputResolutionIndex = Math.max(0, outputResolutionIndex - 1);
    outputWidth = OUTPUT_RESOLUTIONS[outputResolutionIndex].width;
    outputHeight = OUTPUT_RESOLUTIONS[outputResolutionIndex].height;
    applyOutputSize();
});


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

window.addEventListener("toggleConstellation", ()=>{});
window.addEventListener("constellationDistanceUp", ()=>{});
window.addEventListener("constellationDistanceDown", ()=>{});
window.addEventListener("toggleRingsEnabled", ()=>{});
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

    applyOutputSize();
    reportSelection("video");
    requestAnimationFrame(loop);
}


boot();
