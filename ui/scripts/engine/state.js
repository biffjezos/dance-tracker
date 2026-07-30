export const state = {
    wasmApp: null,
    camera: null,
    nextNumberByKind: {},
    cameraActivated: false,
    cameraOn: false,
    videoLayers: [],
    nodes: [],
    selectedVideoId: null,
    outputEntryId: null,
    transportPlaying: false,
    outputResolutionIndex: 0,
    outputWidth: 320,
    outputHeight: 240
};

// Every kind (video, image, camera, and any create_node-backed operation)
// numbers its own layers independently, starting at 1 - never a shared
// counter that makes one kind's first instance look like a later one.
export function nextNumber(kind) {
    const number = (state.nextNumberByKind[kind] || 0) + 1;
    state.nextNumberByKind[kind] = number;
    return number;
}
