/*
==================================================
DANCE TRACKER 5000
CAMERA ENGINE
==================================================
*/

import { logger } from "../core/log.js";

// Module-level singleton: app.js constructs exactly one Camera at boot,
// and getCamera() below is how everyone else (features/video.js) reaches
// that same instance - there is only ever one call site for `new Camera()`.
let cameraInstance = null;

export class Camera {
    constructor(settings, video) {
        this.settings = settings;
        this.video = video || document.getElementById("camera");
        this.stream = null;

        cameraInstance = this;
    }

    getVideo() {
        return this.video;
    }

    async start() {
        logger.debug("Camera start requested");

        try {
            this.stream = await navigator.mediaDevices.getUserMedia({
                video: {
                    width: this.settings.video.width,
                    height: this.settings.video.height
                },
                audio: false
            });

            this.video.srcObject = this.stream;

            this.video.onloadedmetadata = () => {
                logger.debug("Camera video size:", this.video.videoWidth, this.video.videoHeight);
                this.video.play();
            };

        } catch (error) {
            logger.error("Camera failed:", error.name, error.message);
        }
    }

    stop() {
        if (this.stream) {
            this.stream.getTracks().forEach(track => track.stop());
            this.stream = null;
        }
    }
}

export function getCamera() {
    return cameraInstance;
}
