/*
==================================================
WASM INSTANCE WRAPPER

Owns the JavaScript-side lifetime of the Rust/WASM application.

The generated package in core/pkg/dance_tracker_core.js is untouched.
This file only initializes it and provides access to the single App
instance used by graph/render/output modules.
==================================================
*/

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
