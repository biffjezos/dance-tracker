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

/*
==================================================
INITIALIZE APPLICATION
==================================================
*/

const camera = new Camera();
const background = new BackgroundCapture();
const segmentation = new Segmentation(background);
const settings = new Settings();
const menu = new MenuManager();
const renderer = new Renderer({
    settings: settings
});

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
    requestAnimationFrame(processBody);
}
processBody();

window.addEventListener( "startCamera", ()=>{ camera.start(); } );
window.addEventListener( "stopCamera", ()=>{ camera.stop(); } );
window.addEventListener( "captureBackground", ()=>{ background.capture( camera.getVideo() ); }

);
/*
==================================================
STATUS
==================================================
*/
console.log("Dance Tracker 5000 initialized");