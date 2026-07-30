/*
==================================================
VIDEO TRANSPORT

Minimal, generalizable playback control for anything backed by a real
HTMLVideoElement (a loaded video file or a camera stream). Only
play/stop/rewindToStart are implemented for now, on purpose - full
scrub/seek belongs to the timeline/keyframing work, which is a separate,
much bigger piece (see CLAUDE.md's wishlist). This module exists so that
future work has one place to extend rather than reinventing playback
control per call site.
==================================================
*/

export function play(videoEl) {
    if (!videoEl) return;
    videoEl.play().catch(() => {
        // Autoplay/interaction restrictions - not fatal, just don't play.
    });
}

export function stop(videoEl) {
    if (!videoEl) return;
    videoEl.pause();
}

export function rewindToStart(videoEl) {
    if (!videoEl) return;
    videoEl.currentTime = 0;
}

export function togglePlayback(videoEl) {
    if (!videoEl) return;
    if (videoEl.paused) {
        play(videoEl);
    } else {
        stop(videoEl);
    }
}
