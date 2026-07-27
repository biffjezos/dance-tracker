/*
==================================================
UNIVERSAL ROW: stepper / visibility / background / masked by
==================================================
*/

import { state } from "./state.js";

import {
    getVideoRegistry,
    getMaskRegistry,
    getAllRealEntries,
    scopedEntry
} from "./registry.js";

import {
    rebuildGraph,
    updateOutputNodeId
} from "./graph.js";

import { reportSelection } from "./status.js";



function stepSelection(registry, currentId, direction){

    const currentIndex = registry.findIndex(
        entry=>entry.id === currentId
    );


    const nextIndex = Math.min(
        Math.max(
            (currentIndex < 0 ? 0 : currentIndex) + direction,
            0
        ),
        registry.length - 1
    );


    const next = registry[nextIndex];

    return next ? next.id : null;

}



window.addEventListener("videoIndexStep", e=>{

    state.selectedVideoId =
        stepSelection(
            getVideoRegistry(),
            state.selectedVideoId,
            e.detail.direction
        );


    reportSelection("video");

});



window.addEventListener("maskIndexStep", e=>{

    state.selectedMaskId =
        stepSelection(
            getMaskRegistry(),
            state.selectedMaskId,
            e.detail.direction
        );


    reportSelection("mask");

});



window.addEventListener("cycleVisibilityMode", e=>{

    const entry = scopedEntry(e.detail.scope);


    if(!entry.id)
        return;


    state.outputEntryId =
        state.outputEntryId === entry.id
        ? null
        : entry.id;


    updateOutputNodeId();

    reportSelection(e.detail.scope);

});



window.addEventListener("maskSourceStep", e=>{

    const entry = scopedEntry(e.detail.scope);


    const target = entry.layer.settings.maskedBy;


    const ids = [
        "none",
        ...getAllRealEntries()
            .filter(o=>o.id !== entry.id)
            .map(o=>o.id)
    ];


    let index = ids.indexOf(target.source);


    if(index < 0)
        index = 0;


    index = Math.min(
        Math.max(
            index + e.detail.direction,
            0
        ),
        ids.length - 1
    );


    target.source = ids[index];


    rebuildGraph();

    reportSelection(e.detail.scope);

});



window.addEventListener("maskChannelStep", e=>{

    const entry = scopedEntry(e.detail.scope);


    const target = entry.layer.settings.maskedBy;


    const channels = [
        "red",
        "green",
        "blue",
        "alpha"
    ];


    let index = channels.indexOf(target.channel);


    if(index < 0)
        index = 0;


    index = Math.min(
        Math.max(
            index + e.detail.direction,
            0
        ),
        channels.length - 1
    );


    target.channel = channels[index];


    rebuildGraph();

    reportSelection(e.detail.scope);

});