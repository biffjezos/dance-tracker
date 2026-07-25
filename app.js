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



const rings =
    new Rings(
        settings
    );



const ghost =
    new Ghost(
        settings
    );



const text =
    new Text(
        settings
    );



const recorder =
    new Recorder(
        document.getElementById(
            "master-layer"
        )
    );




menu.init();


renderer.start();


document
.querySelector(".statusbar")
.children[2]
.innerText =
    "BODY: " +
    (
        settings.layers.body
        ?
        "ON"
        :
        "OFF"
    );


document
.querySelector(".statusbar")
.children[3]
.innerText =
    "MATTE: " +
    (
        settings.body.mode === "keying"
        ?
        "CHROMA"
        :
        "DIFFERENCE"
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



    ghost.draw();

    rings.draw();

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

        videoLayers.push(
            layer
        );

        layer.loadVideoFile(
            e.detail.file
        );

        console.log(
            "Added",
            layer.name,
            "- total video layers:",
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
VIDEO
==================================================
*/


window.addEventListener(
    "toggleVideo",
    ()=>{


        settings.video.enabled =
        !settings.video.enabled;


        console.log(
            "Video:",
            settings.video.enabled
        );

    }
);



window.addEventListener(
    "toggleVideoVisible",
    ()=>{

        settings.video.visible =
        !settings.video.visible;

        console.log(
            "Video visible:",
            settings.video.visible
        );

    }
);



window.addEventListener(
    "videoBackgroundColour",
    e=>{


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




/*
==================================================
BODY
==================================================
*/


window.addEventListener(
    "toggleBody",
    ()=>{


        settings.layers.body =
        !settings.layers.body;


        console.log(
            "Body:",
            settings.layers.body
        );


        document
        .querySelector(".statusbar")
        .children[2]
        .innerText =
            "BODY: " +
            (
                settings.layers.body
                ?
                "ON"
                :
                "OFF"
            );


    }
);



window.addEventListener(
    "toggleBodyVisible",
    ()=>{

        settings.body.visible =
        !settings.body.visible;

        console.log(
            "Body visible:",
            settings.body.visible
        );

    }
);




window.addEventListener(
    "captureBackground",
    ()=>{

        background.capture(
            camera.getVideo()
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


        settings.body.threshold += 5;


        console.log(
            "Threshold:",
            settings.body.threshold
        );


    }
);



window.addEventListener(
    "thresholdDown",
    ()=>{


        settings.body.threshold -= 5;


        if(settings.body.threshold < 0)
            settings.body.threshold = 0;



        console.log(
            "Threshold:",
            settings.body.threshold
        );


    }
);



window.addEventListener(
    "toggleMatteMode",
    ()=>{

        settings.body.mode =
            settings.body.mode === "difference"
            ?
            "keying"
            :
            "difference";


        console.log(
            "Body matte mode:",
            settings.body.mode
        );


        document
        .querySelector(".statusbar")
        .children[3]
        .innerText =
            "MATTE: " +
            (
                settings.body.mode === "keying"
                ?
                "CHROMA"
                :
                "DIFFERENCE"
            );

    }
);



window.addEventListener(
    "toggleBodyFill",
    ()=>{

        settings.body.fill =
            settings.body.fill === "solid"
            ?
            "video"
            :
            "solid";


        console.log(
            "Body fill:",
            settings.body.fill
        );

    }
);




/*
==================================================
COLOUR
==================================================
*/


window.addEventListener(
    "bodyColour",
    e=>{


        segmentation.setColour(
            e.detail.r,
            e.detail.g,
            e.detail.b
        );


    }
);



window.addEventListener(
    "bodyKeyColour",
    e=>{

        settings.body.keyColour = {

            r:e.detail.r,

            g:e.detail.g,

            b:e.detail.b

        };


        console.log(
            "Body key colour:",
            settings.body.keyColour
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
==================================================
*/


window.addEventListener(
    "toggleRings",
    ()=>{

        settings.amiga.rings.enabled =
        !settings.amiga.rings.enabled;

        console.log(
            "Rings:",
            settings.amiga.rings.enabled
        );

    }
);



window.addEventListener(
    "toggleRingsVisible",
    ()=>{

        settings.amiga.rings.visible =
        !settings.amiga.rings.visible;

        console.log(
            "Rings visible:",
            settings.amiga.rings.visible
        );

    }
);



window.addEventListener(
    "ringCountUp",
    ()=>{

        if(settings.amiga.rings.count < 8)
            settings.amiga.rings.count++;

        console.log(
            "Ring groups:",
            settings.amiga.rings.count
        );

    }
);




window.addEventListener(
    "toggleConstellation",
    ()=>{

        const constellation =
            settings.amiga.rings.constellation;


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

        settings.amiga.rings.constellation.distance += 10;

        console.log(
            "Constellation distance:",
            settings.amiga.rings.constellation.distance
        );

    }
);



window.addEventListener(
    "constellationDistanceDown",
    ()=>{

        settings.amiga.rings.constellation.distance -= 10;

        if(settings.amiga.rings.constellation.distance < 10)
            settings.amiga.rings.constellation.distance = 10;

        console.log(
            "Constellation distance:",
            settings.amiga.rings.constellation.distance
        );

    }
);



window.addEventListener(
    "ringCountDown",
    ()=>{


        if(settings.amiga.rings.count > 1)
            settings.amiga.rings.count--;


        console.log(
            "Ring groups:",
            settings.amiga.rings.count
        );


    }
);



window.addEventListener(
    "ringSizeUp",
    ()=>{

        settings.amiga.rings.size += 10;

    }
);



window.addEventListener(
    "ringSizeDown",
    ()=>{


        if(settings.amiga.rings.size > 20)
            settings.amiga.rings.size -= 10;


    }
);



window.addEventListener(
    "ringThicknessUp",
    ()=>{

        settings.amiga.rings.width += 2;

        console.log(
            "Ring thickness:",
            settings.amiga.rings.width
        );

    }
);



window.addEventListener(
    "ringThicknessDown",
    ()=>{

        settings.amiga.rings.width -= 2;

        if(settings.amiga.rings.width < 1)
            settings.amiga.rings.width = 1;

        console.log(
            "Ring thickness:",
            settings.amiga.rings.width
        );

    }
);



window.addEventListener(
    "ringColour",
    e=>{

        let index =
            e.detail.ringId - 1;


        settings.amiga.rings.colours[index] =
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
            settings.amiga.rings.colours[index]
        );

    }
);




/*
==================================================
GHOSTS
==================================================
*/


window.addEventListener(
    "ghostUp",
    ()=>{


        settings.amiga.ghost.count++;


        settings.amiga.ghost.enabled =
            settings.amiga.ghost.count > 0;



        console.log(
            "Ghost count:",
            settings.amiga.ghost.count
        );


    }
);



window.addEventListener(
    "ghostDown",
    ()=>{


        if(settings.amiga.ghost.count > 0)
            settings.amiga.ghost.count--;



        settings.amiga.ghost.enabled =
            settings.amiga.ghost.count > 0;



        console.log(
            "Ghost count:",
            settings.amiga.ghost.count
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

        settings.amiga.ghost.delay -= 50;

        if(settings.amiga.ghost.delay < 0)
            settings.amiga.ghost.delay = 0;

        console.log(
            "Ghost delay:",
            settings.amiga.ghost.delay
        );

    }
);




/*
==================================================
TEXT
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
    "setTextColour",
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

        settings.amiga.text.size -= 4;

        if(settings.amiga.text.size < 8)
            settings.amiga.text.size = 8;

        console.log(
            "Text size:",
            settings.amiga.text.size
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
MASKING
==================================================
*/


const MASK_SOURCES = [
    "none", "background", "video", "body", "rings", "text"
];


const MASK_CHANNELS = [
    "red", "green", "blue", "alpha"
];


function maskedBySettingsFor(layer){

    if(layer === "video")
        return settings.video.maskedBy;

    if(layer === "body")
        return settings.body.maskedBy;

    if(layer === "rings")
        return settings.amiga.rings.maskedBy;

    if(layer === "text")
        return settings.amiga.text.maskedBy;


    return null;

}


function reportMaskSettingsChanged(layer, target){

    window.dispatchEvent(
        new CustomEvent(
            "maskSettingsChanged",
            {
                detail:{
                    layer:layer,
                    source:target.source,
                    channel:target.channel
                }
            }
        )
    );

}


window.addEventListener(
    "maskSourceStep",
    e=>{

        const layer =
            e.detail.layer;

        const target =
            maskedBySettingsFor(layer);

        if(!target)
            return;


        const validSources =
            MASK_SOURCES.filter(
                s=>s === "none" || s !== layer
            );


        let index =
            validSources.indexOf(
                target.source
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
            validSources[index];


        console.log(
            "Mask source:",
            layer,
            "<-",
            target.source
        );


        reportMaskSettingsChanged(
            layer,
            target
        );

    }
);


window.addEventListener(
    "maskChannelStep",
    e=>{

        const layer =
            e.detail.layer;

        const target =
            maskedBySettingsFor(layer);

        if(!target)
            return;


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
            layer,
            "<-",
            target.channel
        );


        reportMaskSettingsChanged(
            layer,
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