// operations.js
import { wasmApp } from "./wasm.js";

// Callers always pass an operation's own create_node value directly (see
// menu.js's create_node context), never its id - so this only ever needs
// to hand the WASM boundary exactly what it was given.
export function createNode(operationId) {
    return Promise.resolve().then(() => wasmApp.create_node(operationId));
}

export function getOperations() {
    return wasmApp.get_operations();
}

export function executeOperation(id) {
    wasmApp.execute_operation(id);
}
