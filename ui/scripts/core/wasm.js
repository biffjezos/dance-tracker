// ui/scripts/core/wasm.js

import init, { App } from "../../engine/pkg/dance_tracker_engine.js";
import { WIDTH, HEIGHT } from "../engine/constants.js";

export let wasmApp = null;
export let systemMenus = null;
export async function initWasm() {
    if (wasmApp) return wasmApp;

    await init();

    console.log("Creating WASM App");

    wasmApp = App.new(WIDTH, HEIGHT);

    console.log("Created WASM App:", wasmApp);
    systemMenus = wasmApp.get_system_menus();
    return wasmApp;
}

export function getWasmApp() {
    return wasmApp;
}
