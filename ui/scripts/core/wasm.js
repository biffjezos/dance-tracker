// ui/scripts/core/wasm.js

import init, { App } from "../../engine/pkg/dance_tracker_engine.js";
import { WIDTH, HEIGHT } from "../engine/constants.js";

export let wasmApp = null;

export async function initWasm() {
    if (wasmApp) return wasmApp;

    await init();

    wasmApp = new App( WIDTH, HEIGHT );
    return wasmApp;
}

export function getWasmApp() {
    return wasmApp;
}
