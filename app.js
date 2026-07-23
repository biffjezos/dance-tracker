/*
==================================================
DANCE TRACKER 5000
APPLICATION CORE
==================================================
*/

import { BackgroundCapture } from "./body/background.js";
import { Camera } from "./engine/camera.js";
import { Palette } from "./engine/palette.js";
import { MenuManager } from "./engine/menu.js";
import { Settings } from "./engine/settings.js";
import { Renderer } from "./engine/renderer.js";
import { Segmentation } from "./body/segmentation.js";
import { Ghost } from "./effects/ghost.js";
import { Rings } from "./effects/rings.js";
import { Recorder } from "./engine/recorder.js";



const camera =
    new Camera();


const background =
    new BackgroundCapture();


const palette =
    new Palette();


const settings =
    new Settings();



const segmentation =
    new Segmentation(
        background,
        settings,
        palette
    );



const menu =
    new MenuManager();



const renderer =
    new Renderer({
        settings:settings,
        palette:palette
    });



const rings =
    new Rings(
        settings,
        palette
    );



const ghost =
    new Ghost(
        settings,
        palette
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


window.addEventListener(
    "startCamera",
    ()=>{
        camera.start();
    }
);



window.addEventListener(
    "stopCamera",
    ()=>{
        camera.stop();
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
    "ringsOn",
    ()=>{

        settings.amiga.rings.enabled=true;

        console.log(
            "RINGS ON"
        );

    }
);



window.addEventListener(
    "ringsOff",
    ()=>{

        settings.amiga.rings.enabled=false;

        console.log(
            "RINGS OFF"
        );

    }
);



window.addEventListener(
    "ringCountUp",
    ()=>{

        settings.amiga.rings.count++;

        console.log(
            "Ring groups:",
            settings.amiga.rings.count
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




/*
==================================================
RECORDING
==================================================
*/


window.addEventListener(
    "startRecord",
    ()=>{

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
);



window.addEventListener(
    "stopRecord",
    ()=>{

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
);




/*
==================================================
PALETTE
==================================================
*/


window.addEventListener(
    "setPalette",
    e=>{

        palette.set(
            e.detail.name
        );

    }
);



console.log(
    "Dance Tracker 5000 initialized"
);