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
import { Rings } from "./effects/rings.js";

/*
==================================================
INITIALIZE APPLICATION
==================================================
*/

const camera = new Camera();
const background = new BackgroundCapture();
const settings = new Settings();
const segmentation = new Segmentation( background, settings );
const menu = new MenuManager();
const renderer = new Renderer({ settings: settings });
const rings = new Rings(settings);

/*
==================================================
MENU SETUP
==================================================
*/

menu.init();

/*
==================================================
START RENDER LOOP
==================================================
*/
renderer.start();

function processBody(){
    segmentation.process( camera.getVideo() );
    rings.update();
    rings.draw();

    requestAnimationFrame( processBody );
}

processBody();

window.addEventListener( "startCamera", ()=>{ camera.start(); } );
window.addEventListener( "stopCamera", ()=>{ camera.stop(); } );
window.addEventListener( "captureBackground", ()=>{ background.capture( camera.getVideo()); } );
window.addEventListener("thresholdUp", ()=>{

    settings.body.threshold += 5;

    console.log(
        "Threshold:",
        settings.body.threshold
    );

});


window.addEventListener("thresholdDown", ()=>{

    settings.body.threshold -= 5;

    if(settings.body.threshold < 0){
        settings.body.threshold = 0;
    }

    console.log(
        "Threshold:",
        settings.body.threshold
    );

});

window.addEventListener("bodyColour", (event)=>{
    segmentation.setColour(
        event.detail.r,
        event.detail.g,
        event.detail.b
    );
});

window.addEventListener("ringsOn", ()=>{
    settings.amiga.rings.enabled = true;
    console.log("Rings ON");
});


window.addEventListener("ringsOff", ()=>{
    settings.amiga.rings.enabled = false;
    console.log("Rings OFF");
});


window.addEventListener("ringCountUp", ()=>{
    settings.amiga.rings.count++;

    console.log(
        "Ring count:",
        settings.amiga.rings.count
    );
});


window.addEventListener("ringCountDown", ()=>{

    if(settings.amiga.rings.count > 1){
        settings.amiga.rings.count--;
    }

    console.log(
        "Ring count:",
        settings.amiga.rings.count
    );

});


window.addEventListener("ringSizeUp", ()=>{

    settings.amiga.rings.size += 10;

    console.log(
        "Ring size:",
        settings.amiga.rings.size
    );

});


window.addEventListener("ringSizeDown", ()=>{

    if(settings.amiga.rings.size > 20){
        settings.amiga.rings.size -= 10;
    }

    console.log(
        "Ring size:",
        settings.amiga.rings.size
    );

});

window.addEventListener(
    "displayMode",
    e=>{
        settings.video.displayMode =
            e.detail;

        console.log(
            "Display:",
            e.detail
        );
    }
);
/*
==================================================
STATUS
==================================================
*/
console.log("Dance Tracker 5000 initialized");
