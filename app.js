/*
==================================================
DANCE TRACKER 5000
APPLICATION CORE
==================================================
*/

import { BackgroundCapture } from "./body/background.js";
import { Camera } from "./engine/camera.js";
import { MenuManager } from "./engine/menu.js";
import { Settings } from "./engine/settings.js";
import { Renderer } from "./engine/renderer.js";
import { Segmentation } from "./body/segmentation.js";
import { Ghost } from "./effects/ghost.js";
import { Rings } from "./effects/rings.js";
import { Text } from "./effects/text.js";
import { Recorder } from "./engine/recorder.js";
import { containFit } from "./engine/fit.js";
import { VideoLayer } from "./engine/videoLayer.js";
import { MaskLayer } from "./engine/maskLayer.js";



const settings =
    new Settings();


const camera =
    new Camera(
        settings
    );


const background =
    new BackgroundCapture(
        settings
    );



const segmentation =
    new Segmentation(
        background,
        settings
    );



const menu =
    new MenuManager();



const renderer =
    new Renderer({
        settings:settings
    });


const videoLayers = [];

renderer.extraVideoLayers = videoLayers;


const maskLayers = [];

renderer.extraMaskLayers = maskLayers;


function addMaskLayer(){

    const layer =
        new MaskLayer(
            settings,
            {maskNumber:nextMaskNumber++}
        );

    maskLayers.push(layer);

    console.log(
        "Added",
        layer.name,
        "- total standalone masks:",
        maskLayers.length
    );

    return layer;

}


window.addEventListener(
    "addMaskLayer",
    ()=>{

        addMaskLayer();

        reportSelection("mask");

    }
);


function resolveMaskSourceVideo(sourceId){

    if(!sourceId || sourceId === "none")
        return null;

    const match =
        [originalLayerAdapter, ...videoLayers].find(layer=>
            (
                layer.id === "original"
                ?
                "video"
                :
                ("videoLayer:" + layer.id)
            )
            === sourceId
        );

    return match ? match.video : null;

}


function resolveMaskSourceLabel(sourceId){

    if(!sourceId || sourceId === "none")
        return "NONE";

    const match =
        getVideoRegistry().find(entry=>
            entry.kind === "video" &&
            entry.maskSourceId === sourceId
        );

    return match ? match.label : "NONE";

}


/*
The original camera/video/body pair, wrapped in the same shape as a
VideoLayer instance (id/number/name/videoSettings/bodySettings/
rawCanvas/bodyCanvas/video) so future layer-selector code can treat
it uniformly with added layers. videoSettings/bodySettings/rawCanvas/
bodyCanvas/video are direct references to the same live settings
objects and DOM canvases the existing render loop already reads and
writes - not copies - so this stays in sync automatically and changes
nothing about how the original is rendered, toggled, keyed or masked.
Its mask-source id intentionally stays "video"/"body" (not derived
from .id) to match the existing MASKED BY entries from phase 1.
*/
/*
settings.body has no "enabled" field - the original mask's segmentation
is (and always was) gated by the separate settings.layers.body flag,
read directly by both Segmentation.process() and Renderer.compose().
This proxy makes bodySettings.enabled transparently read/write that
same flag so the visibility-mode sync (see cycleVisibilityMode) can
treat every layer uniformly, without actually restructuring that
pre-existing gate.
*/
const originalBodySettings = new Proxy(settings.body, {

    get(target, prop){

        if(prop === "enabled")
            return settings.layers.body;

        return target[prop];

    },

    set(target, prop, value){

        if(prop === "enabled"){

            settings.layers.body = value;

            return true;

        }

        target[prop] = value;

        return true;

    }

});


/*
number/name/maskNumber are assigned dynamically the first time the
camera is turned on (see toggleCamera) - not here, and not hardcoded
to 1. The camera is just another video source; like everything else
in this app, it doesn't exist as a real, addressable thing until the
user actually creates it (turns it on), and whichever real thing
gets created first is the one that becomes "1".
*/
const originalLayerAdapter = {

    id:"original",

    number:null,

    name:null,

    maskNumber:null,

    videoSettings:settings.video,

    bodySettings:originalBodySettings,

    rawCanvas:renderer.layers.effects,

    bodyCanvas:renderer.layers.body,

    video:camera.getVideo(),

    camera:camera,

    background:background,

    segmentation:segmentation

};


/*
Every kind numbers its own instances from 1 (VIDEO 1/2/3, RINGS 1/2,
GHOST 1/2, TEXT 1/2 - each via its own class's self-numbering), so
labels stay small and sequential within whatever list they're shown
in instead of sharing one global counter that leaves gaps whenever
another kind was created in between. Masks are the one kind with no
class of their own (a mask is just a video layer's bundled body
settings, or a standalone MaskLayer) - this counter numbers all of
them, auto or added, in one sequence, starting at 1 - whichever mask
actually gets created first (the camera's or an added video's) earns
MASK 1, nothing is reserved in advance.

nextVideoNumber is the same idea for video-kind entries: the camera
adapter and every VideoLayer draw from this one shared counter in
whatever order they're actually created in (see toggleCamera and
addVideoLayer), instead of the camera always being hardcoded first.
*/
let nextMaskNumber = 1;

let nextVideoNumber = 1;

let cameraActivated = false;


let selectedVideoIndex = 0;

let selectedMaskIndex = 0;


/*
Returned in place of a real entry when nothing real exists yet in
that scope (nothing added, camera never turned on) - same shape as a
real entry so every consumer (status display, universal row
handlers) can keep reading .label/.kind/.settings without a null
check at every call site. label/kind staying null is the signal
menu.js uses to show an empty state instead of a fake node.
*/
const EMPTY_ENTRY = {

    label:null,

    kind:null,

    maskSourceId:null,

    settings:{

        enabled:false,

        visibilityMode:"on",

        maskedBy:{source:"none", channel:"alpha"},

        background:{source:"none", colour:{r:0, g:0, b:0}, blendMode:"normal"}

    }

};


function selectedVideoEntry(){

    const registry =
        getVideoRegistry();

    return (
        registry[selectedVideoIndex] ||
        registry[0] ||
        EMPTY_ENTRY
    );

}


function selectedMaskEntry(){

    const registry =
        getMaskRegistry();

    return (
        registry[selectedMaskIndex] ||
        registry[0] ||
        EMPTY_ENTRY
    );

}


/*
VIDEO and KEY both show the exact same universal row (visibility
mode/background/masked by/transport) - this resolves which of the
two independent steppers a given button click is about, so one
shared set of handlers can serve both instead of duplicating them.
*/
function scopedEntry(scope){

    if(scope === "mask")
        return selectedMaskEntry();

    return selectedVideoEntry();

}


function updateLayerStatusDisplay(entry){

    const nodeDisplay =
        document.getElementById(
            "node-display"
        );

    if(nodeDisplay){

        nodeDisplay.innerText =
            "NODE: " +
            (entry.label || "NONE");

    }


    document
    .querySelector(".statusbar")
    .children[2]
    .innerText =
        "LAYER: " +
        (
            entry.settings.enabled
            ?
            "ON"
            :
            "OFF"
        );


    document
    .querySelector(".statusbar")
    .children[3]
    .innerText =
        "TYPE: " +
        (entry.kind ? entry.kind.toUpperCase() : "NONE");

}


let lastPreviewScope = "video";


function reportSelection(scope){

    lastPreviewScope = scope;

    const entry =
        scopedEntry(scope);

    updateLayerStatusDisplay(
        entry
    );

    reportMaskSettingsChanged(
        scope,
        entry.settings.maskedBy
    );

    reportBackgroundChanged(
        scope,
        entry.settings.background
    );

    window.dispatchEvent(
        new CustomEvent(
            "layerSelectionChanged",
            {
                detail:{
                    scope:scope,
                    label:entry.label,
                    kind:entry.kind,
                    visibilityMode:entry.settings.visibilityMode,
                    keyColour:
                        entry.kind === "mask" ||
                        entry.kind === "standaloneMask"
                        ?
                        entry.settings.keyColour
                        :
                        null,
                    sourceLabel:
                        entry.kind === "standaloneMask"
                        ?
                        resolveMaskSourceLabel(entry.settings.source)
                        :
                        null
                }
            }
        )
    );

}


window.addEventListener(
    "videoIndexStep",
    e=>{

        const count =
            getVideoRegistry().length;

        selectedVideoIndex =
            Math.min(
                Math.max(
                    selectedVideoIndex + e.detail.direction,
                    0
                ),
                count - 1
            );

        reportSelection(
            "video"
        );

    }
);


window.addEventListener(
    "maskIndexStep",
    e=>{

        const count =
            getMaskRegistry().length;

        selectedMaskIndex =
            Math.min(
                Math.max(
                    selectedMaskIndex + e.detail.direction,
                    0
                ),
                count - 1
            );

        reportSelection(
            "mask"
        );

    }
);



const rings =
    new Rings(
        settings
    );

const ringsLayers = [];

renderer.extraRingsLayers = ringsLayers;


function addRingsLayer(){

    const ringsSettings = {

        enabled:true,

        visibilityMode:"on",

        count:2,

        ringsPerGroup:8,

        spacing:14,

        speed:2,

        size:20,

        width:6,

        blend:"screen",

        colours:[
            "rgb(255,0,255)",
            "rgb(0,255,80)",
            "rgb(255,0,255)",
            "rgb(0,255,80)",
            "rgb(255,0,255)",
            "rgb(0,255,80)",
            "rgb(255,0,255)",
            "rgb(0,255,80)"
        ],

        constellation:{

            enabled:false,

            distance:70

        },

        maskedBy:{source:"none", channel:"alpha"},

        background:{source:"none", colour:{r:0, g:0, b:0}, blendMode:"normal"}

    };


    const canvas =
        document.createElement(
            "canvas"
        );

    canvas.width = settings.video.width;

    canvas.height = settings.video.height;


    const layer =
        new Rings(
            settings,
            {
                ringsSettings:ringsSettings,
                outputCanvas:canvas
            }
        );

    ringsLayers.push(layer);


    console.log(
        "Added",
        layer.name,
        "- total rings layers:",
        ringsLayers.length
    );


    return layer;

}


window.addEventListener(
    "addRingsLayer",
    ()=>{

        addRingsLayer();

        reportSelection("video");

    }
);



const ghost =
    new Ghost(
        settings,
        renderer
    );

const ghostLayers = [];


function addGhostLayer(){

    const ghostSettings = {

        enabled:true,

        visibilityMode:"on",

        count:3,

        alpha:0.45,

        delay:50,

        applyToMask:null,

        maskedBy:{source:"none", channel:"alpha"},

        background:{source:"none", colour:{r:0, g:0, b:0}, blendMode:"normal"}

    };


    const canvas =
        document.createElement(
            "canvas"
        );

    canvas.width = settings.video.width;

    canvas.height = settings.video.height;


    const layer =
        new Ghost(
            settings,
            renderer,
            {
                ghostSettings:ghostSettings,
                outputCanvas:canvas
            }
        );

    ghostLayers.push(layer);


    console.log(
        "Added",
        layer.name,
        "- total ghost layers:",
        ghostLayers.length
    );


    return layer;

}


window.addEventListener(
    "addGhostLayer",
    ()=>{

        addGhostLayer();

        reportSelection("video");

    }
);



const text =
    new Text(
        settings
    );

const textLayers = [];


function addTextLayer(){

    const textSettings = {

        content:"",

        colour:"rgb(255,255,255)",

        size:24,

        enabled:true,

        visibilityMode:"on",

        maskedBy:{source:"none", channel:"alpha"},

        background:{source:"none", colour:{r:0, g:0, b:0}, blendMode:"normal"}

    };


    const canvas =
        document.createElement(
            "canvas"
        );

    canvas.width = settings.video.width;

    canvas.height = settings.video.height;


    const layer =
        new Text(
            settings,
            {
                textSettings:textSettings,
                outputCanvas:canvas
            }
        );

    textLayers.push(layer);


    console.log(
        "Added",
        layer.name,
        "- total text layers:",
        textLayers.length
    );


    return layer;

}


window.addEventListener(
    "addTextLayer",
    ()=>{

        addTextLayer();

        reportSelection("video");

    }
);


renderer.extraGhostLayers = ghostLayers;

renderer.extraTextLayers = textLayers;



const recorder =
    new Recorder(
        document.getElementById(
            "master-layer"
        )
    );




menu.init();


renderer.start();


reportSelection("video");

reportSelection("mask");




const cameraPreviewCanvas =
    document.getElementById(
        "camera-preview"
    );

cameraPreviewCanvas.width =
    settings.video.width;

cameraPreviewCanvas.height =
    settings.video.height;

const cameraPreviewCtx =
    cameraPreviewCanvas.getContext(
        "2d"
    );


/*
The CAMERA INPUT panel shows whichever node is currently being
worked on - LIVE OUTPUT is the composition, this is the one thing
selected right now. Resolves via whatever was most recently
navigated: the selected mask's source when working in KEY (its own
owning video for an auto mask, its SOURCE pick for a standalone
one), otherwise the selected VIDEO entry's own previewSource.

previewSource isn't necessarily a <video> - a rings/ghost/text
instance has no camera feed of its own, but it does have its own
rendered canvas (the same one compose() reads), and that's just as
valid a thing to preview. Every registry entry carries its own
previewSource for exactly this reason: whatever kind gets added next
(a static image, some other future node) just needs to set that
field to whatever it draws into, and shows up here for free.
*/
function resolvePreviewSource(){

    if(lastPreviewScope === "mask"){

        const entry =
            selectedMaskEntry();

        /*
        Every other kind's previewSource is fixed for its whole
        lifetime (set once in the registry), but a standalone mask's
        source is a live pick the user can repoint at any time, so
        it's resolved fresh here instead of carrying its own fixed
        previewSource.
        */
        const source =
            entry.kind === "standaloneMask"
            ?
            resolveMaskSourceVideo(
                entry.settings.source
            )
            :
            entry.previewSource;

        if(source)
            return source;

    }
    else {

        const entry =
            selectedVideoEntry();

        if(entry.previewSource)
            return entry.previewSource;

    }

    return camera.getVideo();

}


function drawCameraPreview(){

    const width =
        settings.video.width;

    const height =
        settings.video.height;


    cameraPreviewCtx.clearRect(
        0,
        0,
        width,
        height
    );


    const source =
        resolvePreviewSource();

    if(!source)
        return;


    const isVideo =
        source.tagName === "VIDEO";

    if(isVideo && source.readyState < 2)
        return;


    const sourceWidth =
        isVideo ? source.videoWidth : source.width;

    const sourceHeight =
        isVideo ? source.videoHeight : source.height;

    if(!sourceWidth || !sourceHeight)
        return;


    const rect =
        containFit(
            sourceWidth,
            sourceHeight,
            width,
            height
        );


    cameraPreviewCtx.drawImage(
        source,
        rect.x,
        rect.y,
        rect.width,
        rect.height
    );

}




function processBody(){


    /*
    CLEAR ONLY EFFECT OVERLAYS

    Body layer stays.
    Camera stays.
    Effects redraw cleanly.
    */


    const ringsLayer =
        document.getElementById(
            "rings-layer"
        );

    ringsLayer.getContext("2d").clearRect(
        0,
        0,
        ringsLayer.width,
        ringsLayer.height
    );


    const ghostLayer =
        document.getElementById(
            "ghost-layer"
        );

    ghostLayer.getContext("2d").clearRect(
        0,
        0,
        ghostLayer.width,
        ghostLayer.height
    );


    /*
    Neither Rings nor Ghost clear their own canvas before drawing (Text
    does) - every added instance needs the same external per-frame
    clear the fixed ones get above, or rings would leave trail
    artifacts and ghost's screen-blended history would brighten
    forever instead of actually fading.
    */
    ringsLayers.forEach(layer=>{

        layer.ctx.clearRect(
            0,
            0,
            layer.canvas.width,
            layer.canvas.height
        );

    });


    ghostLayers.forEach(layer=>{

        layer.ctx.clearRect(
            0,
            0,
            layer.canvas.width,
            layer.canvas.height
        );

    });



    segmentation.process(
        camera.getVideo()
    );


    videoLayers.forEach(layer=>{

        layer.update();

    });


    /*
    A standalone mask has no video of its own - it keys against
    whichever real video its SOURCE currently points at, resolved
    fresh every frame since the user can repoint it at any time.
    Nothing to do if it's still set to NONE.
    */
    maskLayers.forEach(layer=>{

        const video =
            resolveMaskSourceVideo(
                layer.bodySettings.source
            );

        if(video)
            layer.segmentation.process(
                video
            );

    });


    ghost.update();

    rings.update();

    ringsLayers.forEach(layer=>{

        layer.update();

    });

    ghostLayers.forEach(layer=>{

        layer.update();

    });



    ghost.draw();

    rings.draw();

    ringsLayers.forEach(layer=>{

        layer.draw();

    });

    ghostLayers.forEach(layer=>{

        layer.draw();

    });

    text.draw();

    textLayers.forEach(layer=>{

        layer.draw();

    });


    /*
    Runs last, after every kind's own draw() for this frame has
    already happened - whichever canvas resolvePreviewSource() picks
    (a generator's own canvas included) needs to already have this
    frame's content in it, not last frame's or a just-cleared blank.
    */
    drawCameraPreview();


    updateTransportDisplay();

    updateAudioSyncDisplay();



    requestAnimationFrame(
        processBody
    );

}


processBody();




/*
==================================================
CAMERA
==================================================
*/


let cameraOn = false;


window.addEventListener(
    "toggleCamera",
    ()=>{

        cameraOn = !cameraOn;


        if(cameraOn){

            if(!cameraActivated){

                cameraActivated = true;

                originalLayerAdapter.number =
                    nextVideoNumber++;

                originalLayerAdapter.name =
                    "VIDEO " + originalLayerAdapter.number;

                originalLayerAdapter.maskNumber =
                    nextMaskNumber++;

                reportSelection("video");

                reportSelection("mask");

            }

            fileSourceActive = false;

            transportPlaying = true;

            camera.start();

        }
        else {

            camera.stop();

        }


        console.log(
            "Camera:",
            cameraOn
        );

    }
);




/*
==================================================
LOAD VIDEO
==================================================
*/


let loadedVideoUrl = null;


window.addEventListener(
    "loadVideoFile",
    e=>{

        if(cameraOn){

            camera.stop();

            cameraOn = false;

        }


        const video =
            camera.getVideo();


        if(loadedVideoUrl){

            URL.revokeObjectURL(
                loadedVideoUrl
            );

        }


        loadedVideoUrl =
            URL.createObjectURL(
                e.detail.file
            );


        video.onerror = ()=>{

            console.error(
                "VIDEO FILE FAILED TO LOAD:",
                e.detail.file.name
            );

        };


        video.srcObject = null;

        video.loop = true;

        video.src = loadedVideoUrl;

        video.play();


        fileSourceActive = true;

        transportPlaying = true;


        console.log(
            "Loaded video file:",
            e.detail.file.name
        );

    }
);



window.addEventListener(
    "addVideoLayer",
    e=>{

        const layer =
            new VideoLayer(
                settings,
                {number:nextVideoNumber++}
            );

        layer.maskNumber =
            nextMaskNumber++;

        videoLayers.push(
            layer
        );

        layer.loadVideoFile(
            e.detail.file
        );

        console.log(
            "Added",
            layer.name,
            "(mask", layer.maskNumber + ") - total video layers:",
            videoLayers.length
        );

        reportSelection("video");

        reportSelection("mask");

    }
);




/*
==================================================
RESOLUTION
==================================================
*/


const RESOLUTIONS = [
    {width:320, height:240},
    {width:640, height:480},
    {width:1280, height:960},
    {width:640, height:360},
    {width:1280, height:720},
    {width:1920, height:1080}
];


let resolutionIndex = 0;


function applyResolution(){


    const res =
        RESOLUTIONS[resolutionIndex];


    settings.video.width = res.width;

    settings.video.height = res.height;


    renderer.resize();

    segmentation.resize();

    background.resize();


    cameraPreviewCanvas.width =
        settings.video.width;

    cameraPreviewCanvas.height =
        settings.video.height;


    videoLayers.forEach(layer=>{

        layer.resize();

    });


    maskLayers.forEach(layer=>{

        layer.resize();

    });


    ringsLayers.forEach(layer=>{

        layer.resize();

    });


    ghostLayers.forEach(layer=>{

        layer.resize();

    });


    textLayers.forEach(layer=>{

        layer.resize();

    });


    document
    .querySelector(".statusbar")
    .children[5]
    .innerText =
        res.width + "x" + res.height;


    console.log(
        "Resolution:",
        res.width,
        res.height
    );


    if(cameraOn){

        camera.stop();

        camera.start();

    }


}


window.addEventListener(
    "outputSizeUp",
    ()=>{

        if(resolutionIndex < RESOLUTIONS.length - 1){

            resolutionIndex++;

            applyResolution();

        }

    }
);


window.addEventListener(
    "outputSizeDown",
    ()=>{

        if(resolutionIndex > 0){

            resolutionIndex--;

            applyResolution();

        }

    }
);




/*
==================================================
LAYER

Universal controls, shared identically by VIDEO and KEY (each keeps
its own selected index/registry, "scope" in the event detail just
says which one a given click was about) - no per-kind branching.
toggleLayerEnabled is unused by any menu now (visibility mode
replaced it) but stays wired in case it's needed again.
==================================================
*/


window.addEventListener(
    "toggleLayerEnabled",
    e=>{

        const entry =
            scopedEntry(e.detail.scope);

        entry.settings.enabled =
        !entry.settings.enabled;


        console.log(
            entry.label,
            "enabled:",
            entry.settings.enabled
        );


        updateLayerStatusDisplay(
            entry
        );

    }
);



/*
Replaces the old plain visible boolean. One button, four states,
cycled forward on each click. "off" also drops the .enabled
computation gate (segmentation etc. stop running) since nothing
manually re-enables it otherwise now that ON/OFF isn't its own
control - every other mode implies "yes, compute this".
*/
const VISIBILITY_MODES = [
    "on", "alpha", "maskWhite", "off"
];


window.addEventListener(
    "cycleVisibilityMode",
    e=>{

        const entry =
            scopedEntry(e.detail.scope);

        let index =
            VISIBILITY_MODES.indexOf(
                entry.settings.visibilityMode
            );

        if(index < 0)
            index = 0;

        index =
            (index + 1) %
            VISIBILITY_MODES.length;

        entry.settings.visibilityMode =
            VISIBILITY_MODES[index];

        entry.settings.enabled =
            entry.settings.visibilityMode !== "off";

        console.log(
            entry.label,
            "visibility mode:",
            entry.settings.visibilityMode
        );

        reportSelection(
            e.detail.scope
        );

    }
);



window.addEventListener(
    "videoBackgroundColour",
    e=>{

        /*
        Scene-wide backdrop fill, not per-layer - there is only one
        background-layer canvas and Renderer.drawBackground() always
        reads settings.video.backgroundColour directly, regardless of
        which layer is selected.
        */

        settings.video.backgroundColour =
            "rgb("
            +
            e.detail.r
            +
            ","
            +
            e.detail.g
            +
            ","
            +
            e.detail.b
            +
            ")";


        console.log(
            "Background colour:",
            settings.video.backgroundColour
        );

    }
);



window.addEventListener(
    "captureLayerBackground",
    ()=>{

        const entry =
            selectedMaskEntry();

        if(!entry.background)
            return;

        const video =
            entry.kind === "standaloneMask"
            ?
            resolveMaskSourceVideo(entry.settings.source)
            :
            entry.video;

        if(!video)
            return;

        entry.background.capture(
            video
        );

    }
);


/*
Cycles a standalone mask's SOURCE (which video's pixels it keys
against) through every real video-kind entry - "NONE" plus each one,
in registry order. Only meaningful for kind "standaloneMask" (a
video-bundled mask's source is always its own owning video); a no-op
otherwise so the button is harmless if ever clicked while some other
mask is selected.
*/
window.addEventListener(
    "maskVideoSourceStep",
    e=>{

        const entry =
            selectedMaskEntry();

        if(entry.kind !== "standaloneMask")
            return;

        const ids =
            [
                "none",
                ...getVideoRegistry()
                    .filter(v=>v.kind === "video")
                    .map(v=>v.maskSourceId)
            ];

        let index =
            ids.indexOf(
                entry.settings.source
            );

        if(index === -1)
            index = 0;

        index =
            Math.min(
                Math.max(
                    index + e.detail.direction,
                    0
                ),
                ids.length - 1
            );

        entry.settings.source =
            ids[index];

        console.log(
            "Mask source:",
            entry.settings.source
        );

        reportSelection("mask");

    }
);




/*
==================================================
THRESHOLD
==================================================
*/


window.addEventListener(
    "thresholdUp",
    ()=>{


        const maskSettings =
            selectedMaskEntry().settings;

        maskSettings.threshold += 5;


        console.log(
            "Threshold:",
            maskSettings.threshold
        );


    }
);



window.addEventListener(
    "thresholdDown",
    ()=>{


        const maskSettings =
            selectedMaskEntry().settings;

        maskSettings.threshold -= 5;


        if(maskSettings.threshold < 0)
            maskSettings.threshold = 0;



        console.log(
            "Threshold:",
            maskSettings.threshold
        );


    }
);



window.addEventListener(
    "toggleMatteMode",
    ()=>{

        const maskSettings =
            selectedMaskEntry().settings;

        maskSettings.mode =
            maskSettings.mode === "difference"
            ?
            "keying"
            :
            "difference";


        console.log(
            "Matte mode:",
            maskSettings.mode
        );

    }
);



window.addEventListener(
    "toggleLayerFill",
    ()=>{

        const maskSettings =
            selectedMaskEntry().settings;

        maskSettings.fill =
            maskSettings.fill === "solid"
            ?
            "video"
            :
            "solid";


        console.log(
            "Fill:",
            maskSettings.fill
        );

    }
);




/*
==================================================
COLOUR
==================================================
*/


window.addEventListener(
    "layerColour",
    e=>{

        selectedMaskEntry().segmentation.setColour(
            e.detail.r,
            e.detail.g,
            e.detail.b
        );

    }
);



window.addEventListener(
    "bodyKeyColour",
    e=>{

        const maskSettings =
            selectedMaskEntry().settings;

        maskSettings.keyColour = {

            r:e.detail.r,

            g:e.detail.g,

            b:e.detail.b

        };


        console.log(
            "Key colour:",
            maskSettings.keyColour
        );

    }
);



let armedKeyColourPick = false;


/*
Samples from cameraPreviewCanvas, not any one fixed video - that
canvas already shows whatever's currently relevant (see
resolvePreviewSource), so picking a colour from what you're actually
looking at is both simpler and more correct than hardcoding one
particular video regardless of what the panel is showing.
*/
window.addEventListener(
    "armKeyColourPicker",
    ()=>{

        armedKeyColourPick = true;

        cameraPreviewCanvas
        .classList.add(
            "sampling"
        );

        console.log(
            "Click the CAMERA INPUT panel to pick a key colour"
        );

    }
);



cameraPreviewCanvas.addEventListener(
    "click",
    e=>{

        if(!armedKeyColourPick)
            return;

        armedKeyColourPick = false;

        const source = e.target;

        source.classList.remove(
            "sampling"
        );


        const rect =
            source.getBoundingClientRect();


        const fit =
            containFit(
                source.width,
                source.height,
                rect.width,
                rect.height
            );


        const x =
            Math.floor(
                (e.clientX - rect.left - fit.x) / fit.width * source.width
            );

        const y =
            Math.floor(
                (e.clientY - rect.top - fit.y) / fit.height * source.height
            );


        if(
            x < 0 ||
            y < 0 ||
            x >= source.width ||
            y >= source.height
        ){

            console.warn(
                "Clicked outside the video content area"
            );

            return;

        }


        const pixel =
            source
            .getContext("2d")
            .getImageData(
                x,
                y,
                1,
                1
            ).data;


        window.dispatchEvent(
            new CustomEvent(
                "bodyKeyColour",
                {
                    detail:{
                        r:pixel[0],
                        g:pixel[1],
                        b:pixel[2]
                    }
                }
            )
        );

    }
);




/*
==================================================
RINGS

Every rings instance is added via GENERATE > ADD RINGS and edited via
its own EDIT button in VIDEO (see renderGeneratorEditor in menu.js) -
these all operate on whichever rings-kind entry is currently selected
in VIDEO's stepper, never a fixed singleton, since there can be any
number of them.
==================================================
*/


window.addEventListener(
    "ringCountUp",
    ()=>{

        const ringsSettings =
            selectedVideoEntry().settings;

        if(ringsSettings.count < 8)
            ringsSettings.count++;

        console.log(
            "Ring groups:",
            ringsSettings.count
        );

    }
);




window.addEventListener(
    "toggleConstellation",
    ()=>{

        const entry =
            selectedVideoEntry();

        const constellation =
            entry.settings.constellation;


        if(constellation.enabled){

            entry.instance.exitConstellation();

        }


        constellation.enabled =
        !constellation.enabled;


        console.log(
            "Constellation:",
            constellation.enabled
        );

    }
);



window.addEventListener(
    "constellationDistanceUp",
    ()=>{

        selectedVideoEntry().settings.constellation.distance += 10;

        console.log(
            "Constellation distance:",
            selectedVideoEntry().settings.constellation.distance
        );

    }
);



window.addEventListener(
    "constellationDistanceDown",
    ()=>{

        const constellation =
            selectedVideoEntry().settings.constellation;

        constellation.distance -= 10;

        if(constellation.distance < 10)
            constellation.distance = 10;

        console.log(
            "Constellation distance:",
            constellation.distance
        );

    }
);



window.addEventListener(
    "ringCountDown",
    ()=>{

        const ringsSettings =
            selectedVideoEntry().settings;


        if(ringsSettings.count > 1)
            ringsSettings.count--;


        console.log(
            "Ring groups:",
            ringsSettings.count
        );


    }
);



window.addEventListener(
    "ringSizeUp",
    ()=>{

        selectedVideoEntry().settings.size += 10;

    }
);



window.addEventListener(
    "ringSizeDown",
    ()=>{

        const ringsSettings =
            selectedVideoEntry().settings;

        if(ringsSettings.size > 20)
            ringsSettings.size -= 10;

    }
);



window.addEventListener(
    "ringThicknessUp",
    ()=>{

        selectedVideoEntry().settings.width += 2;

        console.log(
            "Ring thickness:",
            selectedVideoEntry().settings.width
        );

    }
);



window.addEventListener(
    "ringThicknessDown",
    ()=>{

        const ringsSettings =
            selectedVideoEntry().settings;

        ringsSettings.width -= 2;

        if(ringsSettings.width < 1)
            ringsSettings.width = 1;

        console.log(
            "Ring thickness:",
            ringsSettings.width
        );

    }
);



window.addEventListener(
    "ringColour",
    e=>{

        let index =
            e.detail.ringId - 1;


        const colours =
            selectedVideoEntry().settings.colours;

        colours[index] =
            "rgb("
            +
            e.detail.r
            +
            ","
            +
            e.detail.g
            +
            ","
            +
            e.detail.b
            +
            ")";


        console.log(
            "Ring",
            e.detail.ringId,
            "colour:",
            colours[index]
        );

    }
);




/*
==================================================
GHOSTS

Every ghost instance is added via GENERATE > ADD GHOST and edited via
its own EDIT button in VIDEO - these operate on whichever ghost-kind
entry is currently selected in VIDEO's stepper. APPLY TO MASK (which
mask feeds THIS instance's trail-history algorithm) is per-instance
too, different from MASKED BY (which masks this instance's own
composited output, reachable via VIDEO's universal row).
==================================================
*/


window.addEventListener(
    "ghostUp",
    ()=>{

        const ghostSettings =
            selectedVideoEntry().settings;

        ghostSettings.count++;


        ghostSettings.enabled =
            ghostSettings.count > 0;



        console.log(
            "Ghost count:",
            ghostSettings.count
        );


    }
);



window.addEventListener(
    "ghostDown",
    ()=>{

        const ghostSettings =
            selectedVideoEntry().settings;

        if(ghostSettings.count > 0)
            ghostSettings.count--;



        ghostSettings.enabled =
            ghostSettings.count > 0;



        console.log(
            "Ghost count:",
            ghostSettings.count
        );


    }
);



window.addEventListener(
    "ghostDelayUp",
    ()=>{

        selectedVideoEntry().settings.delay += 50;

        console.log(
            "Ghost delay:",
            selectedVideoEntry().settings.delay
        );

    }
);



window.addEventListener(
    "ghostDelayDown",
    ()=>{

        const ghostSettings =
            selectedVideoEntry().settings;

        ghostSettings.delay -= 50;

        if(ghostSettings.delay < 0)
            ghostSettings.delay = 0;

        console.log(
            "Ghost delay:",
            ghostSettings.delay
        );

    }
);




function eligibleMaskTargets(){

    return getAllRealEntries().filter(
        entry=>entry.settings.visibilityMode === "maskWhite"
    );

}


/*
menu.js dispatches this synchronously right before it renders the
APPLY TO MASK screen (and mutates the event's own detail object to
read the answer back, rather than a separate broadcast-and-listen
round trip - that would re-trigger a render from inside a render).
Reachable only after selecting a specific ghost instance in VIDEO, so
selectedVideoEntry() is guaranteed to be that instance when this
fires - request.options stays null (menu.js's cue to leave its
current state alone) if that's ever not the case.
*/
window.addEventListener(
    "requestApplyToMaskRefresh",
    e=>{

        const entry =
            selectedVideoEntry();

        if(entry.kind !== "ghost")
            return;

        const ghostSettings =
            entry.settings;

        const eligible =
            eligibleMaskTargets();

        const match =
            eligible.find(
                target=>target.maskSourceId === ghostSettings.applyToMask
            );


        e.detail.label =
            match
            ?
            match.label
            :
            "NONE AVAILABLE";

        e.detail.options =
            eligible.map(target=>({
                id:target.maskSourceId,
                label:target.label
            }));

    }
);


/*
A direct selector, not a stepper - pick one of the masks currently
in MASK WHITE mode by name. Ignores anything that isn't actually
eligible right now (e.g. a stale id from a screen that's been open
since before a mask's mode changed).
*/
window.addEventListener(
    "setApplyToMask",
    e=>{

        const entry =
            selectedVideoEntry();

        if(entry.kind !== "ghost")
            return;


        const eligible =
            eligibleMaskTargets();

        const match =
            eligible.find(
                target=>target.maskSourceId === e.detail.id
            );

        if(!match)
            return;


        entry.settings.applyToMask =
            match.maskSourceId;


        console.log(
            "Ghost apply to mask:",
            match.label
        );

    }
);




/*
==================================================
TEXT

Every text instance is added via GENERATE > ADD TEXT and edited via
its own EDIT button in VIDEO - these operate on whichever text-kind
entry is currently selected in VIDEO's stepper.
==================================================
*/


window.addEventListener(
    "setText",
    e=>{

        selectedVideoEntry().settings.content =
            e.detail.value;

    }
);



window.addEventListener(
    "textSizeUp",
    ()=>{

        selectedVideoEntry().settings.size += 4;

        console.log(
            "Text size:",
            selectedVideoEntry().settings.size
        );

    }
);



window.addEventListener(
    "textSizeDown",
    ()=>{

        const textSettings =
            selectedVideoEntry().settings;

        textSettings.size -= 4;

        if(textSettings.size < 8)
            textSettings.size = 8;

        console.log(
            "Text size:",
            textSettings.size
        );

    }
);



window.addEventListener(
    "textColour",
    e=>{

        const textSettings =
            selectedVideoEntry().settings;

        textSettings.colour =
            "rgb("
            +
            e.detail.r
            +
            ","
            +
            e.detail.g
            +
            ","
            +
            e.detail.b
            +
            ")";

        console.log(
            "Text colour:",
            textSettings.colour
        );

    }
);




/*
==================================================
LOAD AUDIO
==================================================
*/


let audioContext = null;

let currentAudioBuffer = null;

let currentAudioSource = null;

let currentAudioTrack = null;

let audioDestinationNode = null;

let audioOffset = 0;

let audioStartedAt = 0;


function stopAudioSource(){

    if(currentAudioSource){

        try {
            currentAudioSource.stop();
        }
        catch(error){}

        currentAudioSource = null;

    }

}


function startAudioSource(fromTime){

    if(!currentAudioBuffer || !audioDestinationNode)
        return;

    stopAudioSource();


    const source =
        audioContext.createBufferSource();

    source.buffer = currentAudioBuffer;

    source.loop = true;

    source.connect(
        audioContext.destination
    );

    source.connect(
        audioDestinationNode
    );


    const offset =
        Math.max(
            0,
            fromTime % currentAudioBuffer.duration
        );

    source.start(0, offset);


    currentAudioSource = source;

    audioStartedAt = audioContext.currentTime;

    audioOffset = fromTime;

}


window.addEventListener(
    "loadAudioFile",
    async e=>{

        try {

            if(!audioContext)
                audioContext = new AudioContext();

            if(audioContext.state === "suspended")
                await audioContext.resume();


            const arrayBuffer =
                await e.detail.file.arrayBuffer();

            currentAudioBuffer =
                await audioContext.decodeAudioData(
                    arrayBuffer
                );


            audioDestinationNode =
                audioContext.createMediaStreamDestination();

            currentAudioTrack =
                audioDestinationNode.stream
                .getAudioTracks()[0];


            if(!hasVideoFile()){

                transportPlaying = true;

            }


            if(transportPlaying){

                startAudioSource(
                    getAudioTargetTime()
                );

            }
            else {

                audioOffset =
                    getPlayheadTime();

            }


            console.log(
                "Loaded audio file:",
                e.detail.file.name
            );

        }
        catch(error){

            console.error(
                "AUDIO FILE FAILED TO LOAD:",
                error.name,
                error.message
            );

        }

    }
);




/*
==================================================
TRANSPORT
==================================================
*/


let fileSourceActive = false;

let transportPlaying = false;


function hasVideoFile(){

    return fileSourceActive;

}


function getPlayheadTime(){

    if(hasVideoFile()){

        return camera.getVideo().currentTime;

    }


    if(transportPlaying && currentAudioBuffer){

        return (
            audioOffset +
            (audioContext.currentTime - audioStartedAt)
        );

    }


    return audioOffset;

}


function formatTimestamp(totalSeconds){

    const pad =
        n=>String(Math.floor(n)).padStart(2, "0");


    totalSeconds += 0.001;


    const minutes =
        Math.floor(totalSeconds / 60);

    const seconds =
        Math.floor(totalSeconds % 60);

    const frames =
        Math.floor((totalSeconds % 1) * 30);


    return (
        pad(minutes) +
        ":" +
        pad(seconds) +
        ":" +
        pad(frames)
    );

}


function updateTransportDisplay(){

    const display =
        document.getElementById(
            "transport-display"
        );

    if(display){

        display.innerText =
            formatTimestamp(
                getPlayheadTime()
            );

    }

}


function seekBy(deltaSeconds){

    const video =
        camera.getVideo();


    if(
        hasVideoFile() &&
        isFinite(video.duration)
    ){

        video.currentTime =
            Math.min(
                Math.max(
                    video.currentTime + deltaSeconds,
                    0
                ),
                video.duration
            );

    }
    else {

        audioOffset =
            Math.max(
                0,
                getPlayheadTime() + deltaSeconds
            );

    }


    if(transportPlaying && currentAudioBuffer){

        startAudioSource(
            getAudioTargetTime()
        );

    }


    updateTransportDisplay();

}


window.addEventListener(
    "transportPlayStop",
    ()=>{

        const video =
            camera.getVideo();


        transportPlaying =
            !transportPlaying;


        if(transportPlaying){

            if(hasVideoFile())
                video.play();

            if(currentAudioBuffer)
                startAudioSource(
                    getAudioTargetTime()
                );

        }
        else {

            if(hasVideoFile())
                video.pause();

            if(currentAudioBuffer){

                audioOffset =
                    getPlayheadTime();

                stopAudioSource();

            }

        }


        console.log(
            "Transport:",
            transportPlaying ? "PLAY" : "STOP"
        );

    }
);


window.addEventListener(
    "transportMinuteUp",
    ()=>seekBy(60)
);


window.addEventListener(
    "transportMinuteDown",
    ()=>seekBy(-60)
);


window.addEventListener(
    "transportSecondUp",
    ()=>seekBy(1)
);


window.addEventListener(
    "transportSecondDown",
    ()=>seekBy(-1)
);


window.addEventListener(
    "transportFrameUp",
    ()=>seekBy(1/30)
);


window.addEventListener(
    "transportFrameDown",
    ()=>seekBy(-1/30)
);




/*
==================================================
AUDIO SYNC
==================================================
*/


let audioSyncOffset = 0;


function getAudioTargetTime(){

    if(hasVideoFile()){

        return (
            camera.getVideo().currentTime +
            audioSyncOffset
        );

    }


    return getPlayheadTime();

}


function formatOffset(totalSeconds){

    const sign =
        totalSeconds < 0
        ?
        "-"
        :
        "+";


    return (
        sign +
        formatTimestamp(
            Math.abs(totalSeconds)
        )
    );

}


function updateAudioSyncDisplay(){

    const display =
        document.getElementById(
            "audio-sync-display"
        );

    if(display){

        display.innerText =
            formatOffset(
                audioSyncOffset
            );

    }

}


function seekAudioSyncBy(deltaSeconds){

    audioSyncOffset +=
        deltaSeconds;


    if(transportPlaying && currentAudioBuffer){

        startAudioSource(
            getAudioTargetTime()
        );

    }


    updateAudioSyncDisplay();

}


window.addEventListener(
    "audioSyncMinuteUp",
    ()=>seekAudioSyncBy(60)
);


window.addEventListener(
    "audioSyncMinuteDown",
    ()=>seekAudioSyncBy(-60)
);


window.addEventListener(
    "audioSyncSecondUp",
    ()=>seekAudioSyncBy(1)
);


window.addEventListener(
    "audioSyncSecondDown",
    ()=>seekAudioSyncBy(-1)
);


window.addEventListener(
    "audioSyncFrameUp",
    ()=>seekAudioSyncBy(1/30)
);


window.addEventListener(
    "audioSyncFrameDown",
    ()=>seekAudioSyncBy(-1/30)
);




/*
==================================================
LAYER REGISTRY

Two independent lists, not one. VIDEO steps over real things only:
every video layer you've actually added, and every generator instance
you've actually added via GENERATE's ADD RINGS/ADD GHOST/ADD TEXT -
the original singleton of each generator kind is deliberately never
listed here (it exists only so renderer.js's fixed-canvas compositing
still has something to read; nothing ever makes it "real" now that
adding is the only way generators come into being, matching "no
default crap"). Once added, an instance stays in the registry for
its whole lifetime regardless of its own visibility mode or enabled
state - those are just properties of a real thing, not a second gate
on top of "does it exist" (a rings instance you've switched to
VISIBILITY: OFF must stay selectable so you can switch it back).
KEY steps over masks only, one per video layer.
==================================================
*/


/*
The camera adapter only belongs in this list once it's actually been
turned on (see toggleCamera) - before that it doesn't exist any more
than an un-added video source does. Sorted by .number rather than
assuming the camera comes first, since whichever video source was
actually created first (camera activation or ADD VIDEO SOURCE) earns
VIDEO 1 - order here just needs to match that, not hardcode it.
*/
function getVideoRegistry(){

    const registry = [];


    const videoSources =
        (
            cameraActivated
            ?
            [originalLayerAdapter]
            :
            []
        )
        .concat(videoLayers)
        .sort((a,b)=>a.number - b.number);


    videoSources.forEach(layer=>{

        const isOriginal =
            layer.id === "original";


        registry.push({
            label:layer.name,
            kind:"video",
            settings:layer.videoSettings,
            maskSourceId:
                isOriginal
                ?
                "video"
                :
                ("videoLayer:" + layer.id),
            video:layer.video,
            previewSource:layer.video
        });

    });


    ringsLayers.forEach(layer=>{

        registry.push({
            label:layer.name,
            kind:"rings",
            settings:layer.ringsSettings,
            maskSourceId:"ringsLayer:" + layer.id,
            instance:layer,
            previewSource:layer.canvas
        });

    });


    textLayers.forEach(layer=>{

        registry.push({
            label:layer.name,
            kind:"text",
            settings:layer.textSettings,
            maskSourceId:"textLayer:" + layer.id,
            instance:layer,
            previewSource:layer.canvas
        });

    });


    ghostLayers.forEach(layer=>{

        registry.push({
            label:layer.name,
            kind:"ghost",
            settings:layer.ghostSettings,
            maskSourceId:"ghostLayer:" + layer.id,
            instance:layer,
            previewSource:layer.canvas
        });

    });


    return registry;

}


function getMaskRegistry(){

    const registry = [];


    const videoSources =
        (
            cameraActivated
            ?
            [originalLayerAdapter]
            :
            []
        )
        .concat(videoLayers)
        .sort((a,b)=>a.maskNumber - b.maskNumber);


    videoSources.forEach(layer=>{

        const isOriginal =
            layer.id === "original";


        registry.push({
            label:"MASK " + layer.maskNumber,
            kind:"mask",
            settings:layer.bodySettings,
            maskSourceId:
                isOriginal
                ?
                "body"
                :
                ("bodyLayer:" + layer.id),
            segmentation:layer.segmentation,
            background:layer.background,
            video:layer.video,
            previewSource:layer.video,
            maskNumber:layer.maskNumber
        });

    });


    maskLayers.forEach(layer=>{

        registry.push({
            label:"MASK " + layer.maskNumber,
            kind:"standaloneMask",
            settings:layer.bodySettings,
            maskSourceId:"maskLayer:" + layer.id,
            segmentation:layer.segmentation,
            background:layer.background,
            instance:layer,
            previewSource:null,
            maskNumber:layer.maskNumber
        });

    });


    /*
    Auto and standalone masks are pushed in two separate passes above,
    but they share one numbering sequence - re-sort the combined list
    by that number so the order shown always matches true creation
    order regardless of which kind happened to be added when.
    */
    registry.sort((a,b)=>a.maskNumber - b.maskNumber);


    return registry;

}


/*
Everything real, video and mask alike, in one list - used only for
mask-source ELIGIBILITY (MASKED BY pickers, Ghost's APPLY TO MASK),
never for stepper navigation. ringsLayers/addRingsLayer still exist
and still render/mask correctly if ever populated, they're just not
reachable from any menu right now ("only Ghost, Rings, Text for
now"), so they're intentionally left out here too.
*/
function getAllRealEntries(){

    return [
        ...getVideoRegistry(),
        ...getMaskRegistry()
    ];

}


/*
==================================================
MASKING
==================================================
*/


const MASK_CHANNELS = [
    "red", "green", "blue", "alpha"
];


function getMaskSources(){

    const sources = [
        {id:"none", label:"NONE"},
        {id:"background", label:"BACKGROUND"}
    ];


    getAllRealEntries().forEach(entry=>{

        sources.push({
            id:entry.maskSourceId,
            label:entry.label
        });

    });


    return sources;

}


function reportMaskSettingsChanged(scope, target){

    const match =
        getMaskSources().find(
            s=>s.id === target.source
        );


    window.dispatchEvent(
        new CustomEvent(
            "maskSettingsChanged",
            {
                detail:{
                    scope:scope,
                    source:target.source,
                    sourceLabel:
                        match
                        ?
                        match.label
                        :
                        target.source.toUpperCase(),
                    channel:target.channel
                }
            }
        )
    );

}


window.addEventListener(
    "maskSourceStep",
    e=>{

        const entry =
            scopedEntry(e.detail.scope);

        const target =
            entry.settings.maskedBy;


        const validSources =
            getMaskSources().filter(
                s=>s.id === "none" || s.id !== entry.maskSourceId
            );


        let index =
            validSources.findIndex(
                s=>s.id === target.source
            );

        if(index < 0)
            index = 0;


        index =
            Math.min(
                Math.max(
                    index + e.detail.direction,
                    0
                ),
                validSources.length - 1
            );


        target.source =
            validSources[index].id;


        console.log(
            "Mask source:",
            entry.label,
            "<-",
            validSources[index].label
        );


        reportMaskSettingsChanged(
            e.detail.scope,
            target
        );

    }
);


window.addEventListener(
    "maskChannelStep",
    e=>{

        const target =
            scopedEntry(e.detail.scope).settings.maskedBy;


        let index =
            MASK_CHANNELS.indexOf(
                target.channel
            );

        if(index < 0)
            index = 0;


        index =
            Math.min(
                Math.max(
                    index + e.detail.direction,
                    0
                ),
                MASK_CHANNELS.length - 1
            );


        target.channel =
            MASK_CHANNELS[index];


        console.log(
            "Mask channel:",
            target.channel
        );


        reportMaskSettingsChanged(
            e.detail.scope,
            target
        );

    }
);




/*
==================================================
BACKGROUND

Per-layer, not scene-wide: what shows through wherever a layer's own
content is transparent - either a flat colour or another real
layer's own fully-resolved appearance, following that layer's own
background if it has one too (renderer.js's resolveChainedLayer does
the recursion, with cycle detection - A's background can be B, B's
background can be C, and a chain that loops back on itself falls back
to the cyclic layer's own un-backgrounded appearance instead of
hanging). Shares the scoped selector/stepper pattern with MASKED BY
(scope "video" or "mask"; a rings/ghost/text instance's background is
also reached this way, through the video scope, since that's where
those instances live in the registry).
==================================================
*/


function getBackgroundSources(){

    const sources = [
        {id:"none", label:"NONE"},
        {id:"colour", label:"COLOUR"}
    ];


    getAllRealEntries().forEach(entry=>{

        sources.push({
            id:entry.maskSourceId,
            label:entry.label
        });

    });


    return sources;

}


function reportBackgroundChanged(scope, target){

    const match =
        getBackgroundSources().find(
            s=>s.id === target.source
        );


    window.dispatchEvent(
        new CustomEvent(
            "backgroundSettingsChanged",
            {
                detail:{
                    scope:scope,
                    source:target.source,
                    sourceLabel:
                        match
                        ?
                        match.label
                        :
                        target.source.toUpperCase(),
                    colour:target.colour,
                    blendMode:target.blendMode || "normal"
                }
            }
        )
    );

}


const BACKGROUND_BLEND_MODES = [
    "normal",
    "overlay",
    "screen"
];


window.addEventListener(
    "backgroundBlendModeStep",
    e=>{

        const entry =
            scopedEntry(e.detail.scope);

        const target =
            entry.settings.background;


        let index =
            BACKGROUND_BLEND_MODES.indexOf(
                target.blendMode || "normal"
            );

        if(index < 0)
            index = 0;


        index =
            Math.min(
                Math.max(
                    index + e.detail.direction,
                    0
                ),
                BACKGROUND_BLEND_MODES.length - 1
            );


        target.blendMode =
            BACKGROUND_BLEND_MODES[index];


        console.log(
            "Background blend mode:",
            entry.label,
            "<-",
            target.blendMode
        );


        reportBackgroundChanged(
            e.detail.scope,
            target
        );

    }
);


window.addEventListener(
    "backgroundSourceStep",
    e=>{

        const entry =
            scopedEntry(e.detail.scope);

        const target =
            entry.settings.background;


        const validSources =
            getBackgroundSources().filter(
                s=>
                    s.id === "none" ||
                    s.id === "colour" ||
                    s.id !== entry.maskSourceId
            );


        let index =
            validSources.findIndex(
                s=>s.id === target.source
            );

        if(index < 0)
            index = 0;


        index =
            Math.min(
                Math.max(
                    index + e.detail.direction,
                    0
                ),
                validSources.length - 1
            );


        target.source =
            validSources[index].id;


        console.log(
            "Background source:",
            entry.label,
            "<-",
            validSources[index].label
        );


        reportBackgroundChanged(
            e.detail.scope,
            target
        );

    }
);


window.addEventListener(
    "layerBackgroundColour",
    e=>{

        const entry =
            scopedEntry(e.detail.scope);

        entry.settings.background.colour = {

            r:e.detail.r,

            g:e.detail.g,

            b:e.detail.b

        };


        console.log(
            "Background colour:",
            entry.settings.background.colour
        );

    }
);




/*
==================================================
RECORDING
==================================================
*/


window.addEventListener(
    "toggleRecord",
    ()=>{

        if(recorder.recording){

            let stopped =
                recorder.stop();

            document
            .querySelector(".statusbar")
            .children[4]
            .innerText =
                stopped
                ?
                "REC: OFF"
                :
                "REC: ERROR";

        }
        else {

            let started =
                recorder.start(currentAudioTrack);

            document
            .querySelector(".statusbar")
            .children[4]
            .innerText =
                started
                ?
                "REC: ON"
                :
                "REC: ERROR";

        }

    }
);




console.log(
    "Dance Tracker 5000 initialized"
);