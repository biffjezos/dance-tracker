/*
==================================================
INPUT: IMAGE FILE
==================================================
*/
import {
    state
} from "../engine/state.js";
import {
    rebuildGraph
} from "../engine/graph.js";
import {
    reportSelection
} from "../engine/status.js";
import {
    defaultUniversalSettings
} from "../state/registry.js";
import {
    getWasmApp
} from "../core/wasm.js";

// Handle the open_image_picker action from menu
window.addEventListener("menuOperation", e => {
    if (e.detail !== "open_image_picker") return;

    // Create file input element
    const input = document.createElement("input");
    input.type = "file";
    input.accept = "image/*";
    input.style.display = "none";

    // Handle file selection
    input.addEventListener("change", async (event) => {
        const file = event.target.files?.[0];
        if (!file) return;

        try {
            // Read the image file and extract pixel data
            const imageData = await readImageFile(file);
            
            // Create image source node in WASM
            const wasmApp = getWasmApp();
            if (!wasmApp) {
                console.error("WASM app not initialized");
                return;
            }

            // Create the node
            const nodeId = wasmApp.create_image_source_node();
            
            // Set the image data on the node
            wasmApp.set_image_on_node(nodeId, new Uint8Array(imageData.pixels), imageData.width, imageData.height);
            
            // Create a layer for this image
            const number = state.nextVideoNumber++;
            const layer = {
                id: "image-" + number,
                number,
                name: "IMAGE " + number,
                imageNodeId: nodeId,
                settings: defaultUniversalSettings()
            };

            state.videoLayers.push(layer);
            state.transportPlaying = true;

            rebuildGraph();
            reportSelection("video");

        } catch (error) {
            console.error("Error loading image:", error);
        } finally {
            // Clean up
            document.body.removeChild(input);
        }
    });

    // Add to DOM and trigger click
    document.body.appendChild(input);
    input.click();
});

/**
 * Read an image file and extract RGBA pixel data
 * @param {File} file - The image file
 * @returns {Promise<{pixels: Uint8Array, width: number, height: number}>}
 */
async function readImageFile(file) {
    return new Promise((resolve, reject) => {
        const img = new Image();
        const canvas = document.createElement("canvas");
        const ctx = canvas.getContext("2d");

        if (!ctx) {
            reject(new Error("Could not create canvas context"));
            return;
        }

        img.onload = () => {
            try {
                // Set canvas dimensions to match image
                canvas.width = img.width;
                canvas.height = img.height;

                // Draw image onto canvas
                ctx.drawImage(img, 0, 0);

                // Get pixel data
                const imageData = ctx.getImageData(0, 0, canvas.width, canvas.height);
                
                resolve({
                    pixels: imageData.data,
                    width: canvas.width,
                    height: canvas.height
                });
            } catch (error) {
                reject(error);
            }
        };

        img.onerror = () => {
            reject(new Error("Failed to load image"));
        };

        // Create object URL for the image
        img.src = URL.createObjectURL(file);
    });
}
