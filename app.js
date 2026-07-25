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


const originalLayerAdapter = {

    id:"original",

    number:1,

    name:"VIDEO 1",

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
Every addressable thing in the app - a video feed, its derived mask,
a rings/text/ghost generator - gets one number from this single
counter, assigned once at creation and never recomputed, so numbers
stay stable as more layers are added regardless of type. 1-4 are
reserved for what exists from page load (video+mask share 1, rings
original is 2, text is 3, ghost is 4); anything added by the user
starts at 5.
*/
let nextGlobalLayerNumber = 1;


function assignGlobalLayerNumber(target){

    target.globalLayerNumber =
        nextGlobalLayerNumber++;

    return target.globalLayerNumber;

}


assignGlobalLayerNumber(
    originalLayerAdapter
);


let selectedVideoIndex = 0;

let selectedMaskIndex = 0;


function selectedVideoEntry(){

    const registry =
        getVideoRegistry();

    return (
        registry[selectedVideoIndex] ||
        registry[0]
    );

}


function selectedMaskEntry(){

    const registry =
        getMaskRegistry();

    return (
        registry[selectedMaskIndex] ||
        registry[0]
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
            entry.label;

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
        entry.kind.toUpperCase();

}


function reportSelection(scope){

    const entry =
        scopedEntry(scope);

    updateLayerStatusDisplay(
        entry
    );

    reportMaskSettingsChanged(
        scope,
        entry.settings.maskedBy
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
                        entry.kind === "mask"
                        ?
                        entry.settings.keyColour
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

assignGlobalLayerNumber(
    rings
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

        maskedBy:{source:"none", channel:"alpha"}

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

    assignGlobalLayerNumber(
        layer
    );

    ringsLayers.push(layer);


    console.log(
        "Added LAYER",
        layer.globalLayerNumber,
        "(rings) - total rings layers:",
        ringsLayers.length
    );


    return layer;

}


window.addEventListener(
    "addRingsLayer",
    ()=>{

        addRingsLayer();

    }
);



const ghost =
    new Ghost(
        settings,
        renderer
    );


const text =
    new Text(
        settings
    );


/*
TEXT and GHOST stay singleton for now (no ADD action of their own
yet), but still claim one global layer number each, at the same
point in the sequence they'd occupy if they were regular registry
entries - rings, then text, then ghost, matching creation order.
*/
const textLayerNumber =
    nextGlobalLayerNumber++;

const ghostLayerNumber =
    nextGlobalLayerNumber++;



const recorder =
    new Recorder(
        document.getElementById(
            "master-layer"
        )
    );




menu.init();


renderer.start();


updateLayerStatusDisplay(
    selectedVideoEntry()
);




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



    segmentation.process(
        camera.getVideo()
    );


    videoLayers.forEach(layer=>{

        layer.update();

    });


    ghost.update();

    rings.update();

    ringsLayers.forEach(layer=>{

        layer.update();

    });



    ghost.draw();

    rings.draw();

    ringsLayers.forEach(layer=>{

        layer.draw();

    });

    text.draw();


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
                settings
            );

        assignGlobalLayerNumber(
            layer
        );

        videoLayers.push(
            layer
        );

        layer.loadVideoFile(
            e.detail.file
        );

        console.log(
            "Added LAYER",
            layer.globalLayerNumber,
            "(video) - total video layers:",
            videoLayers.length
        );

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


    videoLayers.forEach(layer=>{

        layer.resize();

    });


    ringsLayers.forEach(layer=>{

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

        /*
        Any layer's visibility mode can change whether it's eligible
        for Ghost's APPLY TO MASK list, so keep that reported too -
        otherwise the GENERATE > GHOST screen would only learn about
        it once you click SOURCE +/- there.
        */
        reportApplyToMaskChanged();

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

        entry.background.capture(
            entry.video
        );

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


window.addEventListener(
    "armKeyColourPicker",
    ()=>{

        armedKeyColourPick = true;

        camera.getVideo()
        .classList.add(
            "sampling"
        );

        console.log(
            "Click the CAMERA INPUT video to pick a key colour"
        );

    }
);



camera.getVideo().addEventListener(
    "click",
    e=>{

        if(!armedKeyColourPick)
            return;

        armedKeyColourPick = false;

        const video = e.target;

        video.classList.remove(
            "sampling"
        );


        const rect =
            video.getBoundingClientRect();


        const fit =
            containFit(
                video.videoWidth,
                video.videoHeight,
                rect.width,
                rect.height
            );


        const x =
            Math.floor(
                (e.clientX - rect.left - fit.x) / fit.width * video.videoWidth
            );

        const y =
            Math.floor(
                (e.clientY - rect.top - fit.y) / fit.height * video.videoHeight
            );


        if(
            x < 0 ||
            y < 0 ||
            x >= video.videoWidth ||
            y >= video.videoHeight
        ){

            console.warn(
                "Clicked outside the video content area"
            );

            return;

        }


        const temp =
            document.createElement(
                "canvas"
            );

        temp.width = video.videoWidth;

        temp.height = video.videoHeight;

        const tempCtx =
            temp.getContext("2d");

        tempCtx.drawImage(
            video,
            0,
            0
        );


        const pixel =
            tempCtx.getImageData(
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

GENERATE > RINGS is a fixed, non-steppable screen (no multi-instance
ADD for now), so these read/write the one rings singleton directly.
Once toggleRingsEnabled turns it on, it becomes "real" and appears
as its own entry in VIDEO's registry with the same universal row
(visibility mode/background/masked by/transport) as everything else -
this file section is only the rings-specific deep settings.
==================================================
*/


window.addEventListener(
    "toggleRingsEnabled",
    ()=>{

        rings.ringsSettings.enabled =
        !rings.ringsSettings.enabled;

        console.log(
            "Rings enabled:",
            rings.ringsSettings.enabled
        );

    }
);


window.addEventListener(
    "ringCountUp",
    ()=>{

        const ringsSettings =
            rings.ringsSettings;

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

        const constellation =
            rings.ringsSettings.constellation;


        if(constellation.enabled){

            rings.exitConstellation();

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

        rings.ringsSettings.constellation.distance += 10;

        console.log(
            "Constellation distance:",
            rings.ringsSettings.constellation.distance
        );

    }
);



window.addEventListener(
    "constellationDistanceDown",
    ()=>{

        const constellation =
            rings.ringsSettings.constellation;

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
            rings.ringsSettings;


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

        rings.ringsSettings.size += 10;

    }
);



window.addEventListener(
    "ringSizeDown",
    ()=>{

        const ringsSettings =
            rings.ringsSettings;

        if(ringsSettings.size > 20)
            ringsSettings.size -= 10;

    }
);



window.addEventListener(
    "ringThicknessUp",
    ()=>{

        rings.ringsSettings.width += 2;

        console.log(
            "Ring thickness:",
            rings.ringsSettings.width
        );

    }
);



window.addEventListener(
    "ringThicknessDown",
    ()=>{

        const ringsSettings =
            rings.ringsSettings;

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
            rings.ringsSettings.colours;

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

GENERATE > GHOST is fixed/non-steppable too, so these read/write
settings.amiga.ghost directly. APPLY TO MASK (which mask feeds the
trail-history algorithm) is a different control from MASKED BY
(which masks Ghost's own composited output, still reachable via
VIDEO's universal row when Ghost's entry is selected).
==================================================
*/


window.addEventListener(
    "ghostUp",
    ()=>{

        const ghostSettings =
            settings.amiga.ghost;

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
            settings.amiga.ghost;

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

        settings.amiga.ghost.delay += 50;

        console.log(
            "Ghost delay:",
            settings.amiga.ghost.delay
        );

    }
);



window.addEventListener(
    "ghostDelayDown",
    ()=>{

        const ghostSettings =
            settings.amiga.ghost;

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


function reportApplyToMaskChanged(){

    const ghostSettings =
        settings.amiga.ghost;

    const eligible =
        eligibleMaskTargets();

    const match =
        eligible.find(
            e=>e.maskSourceId === ghostSettings.applyToMask
        );


    window.dispatchEvent(
        new CustomEvent(
            "applyToMaskChanged",
            {
                detail:{
                    id:ghostSettings.applyToMask,
                    label:
                        match
                        ?
                        match.label
                        :
                        "NONE AVAILABLE",
                    options:
                        eligible.map(e=>({
                            id:e.maskSourceId,
                            label:e.label
                        }))
                }
            }
        )
    );

}


/*
A direct selector, not a stepper - pick one of the masks currently
in MASK WHITE mode by name. Ignores anything that isn't actually
eligible right now (e.g. a stale id from a screen that's been open
since before a mask's mode changed).
*/
window.addEventListener(
    "setApplyToMask",
    e=>{

        const eligible =
            eligibleMaskTargets();

        const match =
            eligible.find(
                entry=>entry.maskSourceId === e.detail.id
            );

        if(!match)
            return;


        settings.amiga.ghost.applyToMask =
            match.maskSourceId;


        console.log(
            "Ghost apply to mask:",
            match.label
        );


        reportApplyToMaskChanged();

    }
);


window.addEventListener(
    "applyToMaskStep",
    e=>{

        const ghostSettings =
            settings.amiga.ghost;

        const eligible =
            eligibleMaskTargets();


        if(eligible.length === 0){

            ghostSettings.applyToMask = null;

            reportApplyToMaskChanged();

            return;

        }


        let index =
            eligible.findIndex(
                entry=>entry.maskSourceId === ghostSettings.applyToMask
            );

        if(index < 0)
            index = 0;


        index =
            Math.min(
                Math.max(
                    index + e.detail.direction,
                    0
                ),
                eligible.length - 1
            );


        ghostSettings.applyToMask =
            eligible[index].maskSourceId;


        console.log(
            "Ghost apply to mask:",
            eligible[index].label
        );


        reportApplyToMaskChanged();

    }
);




/*
==================================================
TEXT

GENERATE > TEXT is fixed/non-steppable too, so these read/write
settings.amiga.text directly.
==================================================
*/


window.addEventListener(
    "setText",
    e=>{

        settings.amiga.text.content =
            e.detail.value;

    }
);



window.addEventListener(
    "textSizeUp",
    ()=>{

        settings.amiga.text.size += 4;

        console.log(
            "Text size:",
            settings.amiga.text.size
        );

    }
);



window.addEventListener(
    "textSizeDown",
    ()=>{

        const textSettings =
            settings.amiga.text;

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

        settings.amiga.text.colour =
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
            settings.amiga.text.colour
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

Two independent lists, not one. VIDEO steps over "real" things only -
every video layer you've actually added, plus a generator only once
you've actually done something with it (ghost once its count is
above zero, rings once explicitly turned on, text once it has real
content) - untouched defaults never show up just because the class
instance happens to exist. KEY steps over masks only, one per video
layer. Both share the same entry shape (label/kind/settings with
enabled/visibilityMode/maskedBy), which is what lets one set of
universal handlers serve both.
==================================================
*/


function isGeneratorReal(kind){

    if(kind === "ghost")
        return settings.amiga.ghost.enabled;

    if(kind === "rings")
        return rings.ringsSettings.enabled;

    if(kind === "text")
        return settings.amiga.text.content.trim() !== "";

    return false;

}


function getVideoRegistry(){

    const registry = [];


    [originalLayerAdapter, ...videoLayers].forEach(layer=>{

        const isOriginal =
            layer.id === "original";


        registry.push({
            label:"LAYER " + layer.globalLayerNumber,
            kind:"video",
            settings:layer.videoSettings,
            maskSourceId:
                isOriginal
                ?
                "video"
                :
                ("videoLayer:" + layer.id)
        });

    });


    if(isGeneratorReal("rings")){

        registry.push({
            label:"LAYER " + rings.globalLayerNumber,
            kind:"rings",
            settings:rings.ringsSettings,
            maskSourceId:"rings",
            instance:rings
        });

    }


    if(isGeneratorReal("text")){

        registry.push({
            label:"LAYER " + textLayerNumber,
            kind:"text",
            settings:settings.amiga.text,
            maskSourceId:"text"
        });

    }


    if(isGeneratorReal("ghost")){

        registry.push({
            label:"LAYER " + ghostLayerNumber,
            kind:"ghost",
            settings:settings.amiga.ghost,
            maskSourceId:"ghost"
        });

    }


    return registry;

}


function getMaskRegistry(){

    const registry = [];


    [originalLayerAdapter, ...videoLayers].forEach(layer=>{

        const isOriginal =
            layer.id === "original";


        registry.push({
            label:"MASK " + layer.globalLayerNumber,
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
            video:layer.video
        });

    });


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