/*
==================================================
DANCE TRACKER 5000
VIDEO LAYER

A self-contained, pluggable video source. Multiple
instances can run side by side, each with its own
file/camera and own visibility. Carries no mask of
its own - a mask only ever exists as a standalone
MaskLayer, created solely via ADD MASK.
==================================================
*/


import { Camera } from "./camera.js";
import { containFit } from "./fit.js";


export class VideoLayer {


    constructor(settings, options){


        options = options || {};

        this.settings = settings;


        this.number = options.number;

        this.id = "video-layer-" + this.number;

        this.name = "VIDEO " + this.number;


        this.video =
            document.createElement(
                "video"
            );

        this.video.muted = true;

        this.video.playsInline = true;

        this.video.autoplay = true;


        this.videoSettings = {

            enabled:true,

            visibilityMode:"on",

            maskedBy:{source:"none", channel:"alpha"},

            background:{source:"none", colour:{r:0, g:0, b:0}, blendMode:"normal"}

        };


        this.camera =
            new Camera(
                settings,
                this.video
            );


        this.rawCanvas =
            document.createElement(
                "canvas"
            );

        this.rawCanvas.width = settings.video.width;

        this.rawCanvas.height = settings.video.height;

        this.rawCtx =
            this.rawCanvas.getContext(
                "2d"
            );


        this.fileSourceActive = false;

        this.loadedVideoUrl = null;

    }




    loadVideoFile(file){


        if(this.loadedVideoUrl){

            URL.revokeObjectURL(
                this.loadedVideoUrl
            );

        }


        this.loadedVideoUrl =
            URL.createObjectURL(
                file
            );


        this.video.srcObject = null;

        this.video.loop = true;

        this.video.src =
            this.loadedVideoUrl;

        this.video.play();


        this.fileSourceActive = true;


        console.log(
            "Loaded video file into",
            this.name,
            ":",
            file.name
        );

    }




    resize(){


        this.rawCanvas.width =
            this.settings.video.width;

        this.rawCanvas.height =
            this.settings.video.height;

    }




    drawRawFrame(){


        const width =
            this.settings.video.width;

        const height =
            this.settings.video.height;


        this.rawCtx.clearRect(
            0,
            0,
            width,
            height
        );


        if(!this.videoSettings.enabled)
            return;


        if(this.video.readyState < 2)
            return;


        const rect =
            containFit(
                this.video.videoWidth,
                this.video.videoHeight,
                width,
                height
            );


        this.rawCtx.drawImage(
            this.video,
            rect.x,
            rect.y,
            rect.width,
            rect.height
        );

    }




    update(){


        this.drawRawFrame();

    }


}
