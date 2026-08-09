/*
==================================================
DANCE TRACKER 5000
RECORDING ENGINE
==================================================
*/
import { logger } from "../core/log.js";

export class Recorder {
    constructor(canvas) {
        this.canvas = canvas;
        this.recorder = null;
        this.chunks = [];
        this.recording = false;
    }
    start(audioTrack) {
        if (this.recording) return true;
        try {
            let tracks = this.canvas.captureStream(60).getVideoTracks();
            if (audioTrack) {
                tracks = tracks.concat(
                    [audioTrack]);
            }
            let stream = new MediaStream(tracks);
            // Own, private array for this recording session - deliberately
            // not read back off `this.chunks` inside ondataavailable below.
            // `this.chunks` is still assigned (stop() needs a synchronous
            // way to grab the current session's array), but the handler
            // closure captures `sessionChunks` directly, so a later
            // session's start() reassigning `this.chunks` to a fresh array
            // can never retroactively redirect where this handler writes -
            // see RFC-009 Finding 2.
            const sessionChunks = [];
            this.chunks = sessionChunks;
            let options = {};
            // Explicit codec strings first, not just a bare container
            // check - MediaRecorder.isTypeSupported("video/mp4") can be
            // true while still producing a stream with no clearly-signaled
            // codec or a fragmented (non-faststart) layout that plays back
            // fine in a <video> element but many external players reject.
            // See RFC-009 Finding 1.
            if (MediaRecorder.isTypeSupported("video/mp4;codecs=avc1.42E01E")) {
                options.mimeType = "video/mp4;codecs=avc1.42E01E";
            } else if (MediaRecorder.isTypeSupported("video/mp4;codecs=avc1")) {
                options.mimeType = "video/mp4;codecs=avc1";
            } else if (MediaRecorder.isTypeSupported("video/mp4")) {
                options.mimeType = "video/mp4";
            } else if (MediaRecorder.isTypeSupported("video/webm;codecs=vp9")) {
                options.mimeType = "video/webm;codecs=vp9";
            } else if (MediaRecorder.isTypeSupported("video/webm")) {
                options.mimeType = "video/webm";
            }
            this.recorder = new MediaRecorder(stream, options);
            this.recorder.ondataavailable = event => {
                if (event.data.size > 0) {
                    sessionChunks.push(event.data);
                }
            };
            this.recorder.start();
            this.recording = true;
            logger.debug("Recording started", this.recorder.mimeType);
            return true;
        } catch (error) {
            logger.error("Recording failed to start:", error.name, error.message);
            this.recording = false;
            this.recorder = null;
            return false;
        }
    }
    stop() {
        if (!this.recorder || !this.recording) return true;
        let mimeType = this.recorder.mimeType;
        // Captured synchronously, here, before any later start() call can
        // reassign `this.chunks` to a different session's array - onstop
        // below (firing asynchronously, possibly after a new session has
        // already begun) must build this session's Blob from exactly this
        // reference, never by re-reading `this.chunks` at fire time. See
        // RFC-009 Finding 2.
        const sessionChunks = this.chunks;
        this.recorder.onstop = () => {
            try {
                let blob = new Blob(sessionChunks, {
                    type: mimeType
                });
                let url = URL.createObjectURL(blob);
                let link = document.createElement("a");
                link.href = url;
                let extension = mimeType.includes("mp4") ? "mp4" : "webm";
                link.download = "dance-tracker-recording." + extension;
                document.body.appendChild(link);
                link.click();
                document.body.removeChild(link);
                URL.revokeObjectURL(url);
                logger.debug("Recording saved");
            } catch (error) {
                logger.error("Recording failed to save:", error.name, error.message);
            }
        };
        try {
            this.recorder.stop();
        } catch (error) {
            logger.error("Recording failed to stop:", error.name, error.message);
            this.recording = false;
            this.recorder = null;
            return false;
        }
        this.recording = false;
        this.recorder = null;
        logger.debug("Recording stopped");
        return true;
    }
}