/*
==================================================
DANCE TRACKER 5000
VIDEO LAYER

A self-contained, pluggable video source + its own
body-mask pipeline. Multiple instances can run side
by side, each with its own file/camera, own threshold/
key colour/fill, own visibility.
==================================================
*/


import { Camera } from "./camera.js";
import { BackgroundCapture } from "../body/background.js";
import { Segmentation } from "../body/segmentation.js";
import { containFit } from "./fit.js";


let nextLayerNumber = 2;


export class VideoLayer {


    constructor(settings){


        this.settings = settings;


        this.number = nextLayerNumber++;

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

            background:{source:"none", colour:{r:0, g:0, b:0}}

        };


        this.bodySettings = {

            mode:"difference",

            threshold:100,

            keyColour:{r:0, g:255, b:0},

            fill:"solid",

            enabled:true,

            visibilityMode:"on",

            maskedBy:{source:"none", channel:"alpha"},

            background:{source:"none", colour:{r:0, g:0, b:0}}

        };


        this.camera =
            new Camera(
                settings,
                this.video
            );


        this.background =
            new BackgroundCapture(
                settings
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


        this.bodyCanvas =
            document.createElement(
                "canvas"
            );

        this.bodyCanvas.width = settings.video.width;

        this.bodyCanvas.height = settings.video.height;


        this.segmentation =
            new Segmentation(
                this.background,
                settings,
                {
                    bodySettings:this.bodySettings,
                    outputCanvas:this.bodyCanvas
                }
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




    captureBackground(){


        this.background.capture(
            this.video
        );

    }




    resize(){


        this.rawCanvas.width =
            this.settings.video.width;

        this.rawCanvas.height =
            this.settings.video.height;


        this.background.resize();

        this.segmentation.resize();

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

        this.segmentation.process(
            this.video
        );

    }


}
