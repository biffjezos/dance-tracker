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




function processBody(){


    /*
    CLEAR ONLY EFFECT OVERLAY

    Body layer stays.
    Camera stays.
    Effects redraw cleanly.
    */


    const overlay =
        document.getElementById(
            "overlay-layer"
        );


    const overlayCtx =
        overlay.getContext(
            "2d"
        );


    overlayCtx.clearRect(
        0,
        0,
        overlay.width,
        overlay.height
    );



    segmentation.process(
        camera.getVideo()
    );


    ghost.update();

    rings.update();



    ghost.draw();

    rings.draw();

    text.draw();



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
RESOLUTION
==================================================
*/


const RESOLUTIONS = [
    {width:320, height:240},
    {width:640, height:480},
    {width:1280, height:960}
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


document
.querySelector(".statusbar")
.children[5]
.addEventListener(
    "click",
    ()=>{

        resolutionIndex =
            (resolutionIndex + 1) % RESOLUTIONS.length;

        applyResolution();

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
                recorder.start();

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