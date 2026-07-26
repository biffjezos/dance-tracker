/*
==================================================
DANCE TRACKER 5000
MASK LAYER

A standalone, independently-addable mask. Every video
layer still comes with its own bundled mask for free,
but a MaskLayer is a second way to get one: not tied to
any one video, pointed at whichever video's pixels it
should key against via its own SOURCE stepper, same
threshold/key colour/fill pipeline as a bundled mask.
==================================================
*/


import { BackgroundCapture } from "../body/background.js";
import { Segmentation } from "../body/segmentation.js";


export class MaskLayer {


    constructor(settings, options){


        this.settings = settings;

        this.maskNumber = options.maskNumber;

        this.id = "mask-layer-" + this.maskNumber;

        this.name = "MASK " + this.maskNumber;


        this.bodySettings = {

            mode:"difference",

            threshold:100,

            keyColour:{r:0, g:255, b:0},

            fill:"solid",

            enabled:true,

            visibilityMode:"on",

            source:"none",

            maskedBy:{source:"none", channel:"alpha"},

            background:{source:"none", colour:{r:0, g:0, b:0}, blendMode:"normal"}

        };


        this.canvas =
            document.createElement(
                "canvas"
            );

        this.canvas.width = settings.video.width;

        this.canvas.height = settings.video.height;


        this.background =
            new BackgroundCapture(
                settings
            );


        this.segmentation =
            new Segmentation(
                this.background,
                settings,
                {
                    bodySettings:this.bodySettings,
                    outputCanvas:this.canvas
                }
            );

    }




    resize(){

        this.canvas.width =
            this.settings.video.width;

        this.canvas.height =
            this.settings.video.height;

        this.background.resize();

        this.segmentation.resize();

    }


}
