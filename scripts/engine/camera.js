/*
==================================================
DANCE TRACKER 5000
CAMERA ENGINE
==================================================
*/

let cameraInstance = null;

export class Camera {
    constructor(settings, video) {
        if (cameraInstance) {
            return cameraInstance;
        }

        this.settings = settings;
        this.video = video || document.getElementById("camera");
        this.stream = null;

        cameraInstance = this;
    }

    getVideo() {
        return this.video;
    }

    async start() {
        console.log("CAMERA START REQUESTED");

        try {
            this.stream = await navigator.mediaDevices.getUserMedia({
                video: {
                    width: this.settings.video.width,
                    height: this.settings.video.height
                },
                audio: false
            });

            console.log("STREAM RECEIVED");

            this.video.srcObject = this.stream;

            this.video.onloadedmetadata = () => {
                console.log(
                    "VIDEO SIZE:",
                    this.video.videoWidth,
                    this.video.videoHeight
                );

                this.video.play();
            };

        } catch (error) {
            console.error(
                "CAMERA FAILED:",
                error.name,
                error.message
            );
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
