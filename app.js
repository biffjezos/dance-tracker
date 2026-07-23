/*
==================================================
DANCE TRACKER 5000
APPLICATION CORE
==================================================
*/


import { MenuManager } from "./engine/menu.js";
import { Settings } from "./engine/settings.js";
import { Renderer } from "./engine/renderer.js";





/*
==================================================
INITIALIZE APPLICATION
==================================================
*/


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






/*
==================================================
STATUS
==================================================
*/


console.log(

    "Dance Tracker 5000 initialized"

);