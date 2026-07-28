import "./features/composite.js";
import {
    WIDTH,
    HEIGHT
} from "./engine/constants.js";
import "./features/generators.js";
import "./engine/layerControls.js";
import "./features/masks.js";
import "./features/notWired.js";
import "./features/output.js";
import {
    applyOutputSize
} from "./features/output.js";
import "./features/sources.js";
import {
    startRenderLoop
} from "./engine/render.js";
import {
    initWasm
} from "./core/wasm.js";
import {
    reportSelection
} from "./engine/status.js";
import "./features/transport.js";
import "./features/video.js";
import {
    Camera
} from "./engine/camera.js";
import {
    MenuManager
} from "./engine/menu.js";


const settings = {
    video: {
        width: WIDTH,
        height: HEIGHT
    }
};
const camera = new Camera(settings);
const menu = new MenuManager();
/* BOOT */
menu.init();
document.getElementById("master-layer").width = WIDTH;
document.getElementById("master-layer").height = HEIGHT;
document.getElementById("camera-preview").width = WIDTH;
document.getElementById("camera-preview").height = HEIGHT;
async function boot() {
    await initWasm();
    applyOutputSize();
    reportSelection("video");
    startRenderLoop();
}
boot();