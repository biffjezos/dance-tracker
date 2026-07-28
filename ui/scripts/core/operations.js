// operations.js
import { wasmApp } from "./wasm.js";

export function createNode(operationId) { return wasmApp.create_node(operationId); }
export function getOperations() { return wasmApp.get_operations(); }
export function executeOperation(id) { wasmApp.execute_operation(id); }