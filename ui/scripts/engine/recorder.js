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
            this.chunks = [];
            let options = {};
            if (MediaRecorder.isTypeSupported("video/mp4")) {
                options.mimeType = "video/mp4";
            } else if (MediaRecorder.isTypeSupported("video/webm;codecs=vp9")) {
                options.mimeType = "video/webm;codecs=vp9";
            } else if (MediaRecorder.isTypeSupported("video/webm")) {
                options.mimeType = "video/webm";
            }
            this.recorder = new MediaRecorder(stream, options);
            this.recorder.ondataavailable = event => {
                if (event.data.size > 0) {
                    this.chunks.push(event.data);
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
        this.recorder.onstop = () => {
            try {
                let blob = new Blob(this.chunks, {
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