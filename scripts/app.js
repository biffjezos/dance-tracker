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
mask (MASKED BY), generate (rings/ghost/text), compose, transport, and
record. Audio sync and per-stroke ring CONSTELLATION are not wired up
yet - clicking those buttons is currently a no-op.

Compositing (two nodes drawn together) only ever happens through an
explicit COMPOSITE node the user creates via COMPOSE - there is no
per-node BACKGROUND field anymore, and no automatic stacking of
whatever's independently visible (see CLAUDE.md). The master output is
exactly whichever single node is currently marked VISIBILITY: ON
(outputEntryId, see rebuildGraph); if that's a COMPOSITE, its own
foreground/background wiring is what makes two things appear together.
==================================================
*/
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
async function boot() {
    await initWasm();
    applyOutputSize();
    reportSelection("video");
    startRenderLoop();
}
boot();