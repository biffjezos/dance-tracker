// operations.js
import { wasmApp } from "./wasm.js";

// Map of operation IDs to their create_node values
export const operationMap = {};

export function createNode(operationId) {
    return Promise.resolve().then(() => {
        // Check if this operation has a create_node mapping
        if (operationMap[operationId]) {
            return wasmApp.create_node(operationMap[operationId]);
        }
        return wasmApp.create_node(operationId);
    });
}

export function getOperations() {
    const ops = wasmApp.get_operations();
    
    // Build a map of operation IDs to their create_node values
    ops.forEach(op => {
        if (op.create_node) {
            operationMap[op.id] = op.create_node;
        }
    });
    
    return ops;
}

export function executeOperation(id) {
    wasmApp.execute_operation(id);
}
