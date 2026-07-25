/*
==================================================
DANCE TRACKER 5000
1990s STYLE BODY SEGMENTATION
==================================================
*/


import { containFit } from "../engine/fit.js";


export class Segmentation {


    constructor(background, settings, options){


        options = options || {};


        this.background = background;

        this.settings = settings;


        this.bodySettings =
            options.bodySettings ||
            settings.body;


        this.useGlobalEnabled =
            !options.bodySettings;


        this.canvas =
            document.createElement(
                "canvas"
            );


        this.canvas.width = settings.video.width;

        this.canvas.height = settings.video.height;


        this.ctx =
            this.canvas.getContext(
                "2d"
            );



        this.output =
            options.outputCanvas ||
            document.getElementById(
                "body-layer"
            );


        this.outputCtx =
            this.output.getContext(
                "2d"
            );


        this.colour = {

            r:255,

            g:0,

            b:255

        };


    }





    getColour(){


        return this.colour;


    }




    resize(){


        this.canvas.width =
            this.settings.video.width;


        this.canvas.height =
            this.settings.video.height;


        this.output.width =
            this.settings.video.width;


        this.output.height =
            this.settings.video.height;


    }





    process(video){


        const enabled =
            this.useGlobalEnabled
            ?
            this.settings.layers.body
            :
            this.bodySettings.enabled;


        if(!enabled)
            return;



        const keying =
            this.bodySettings.mode === "keying";



        if(!keying && !this.background.hasBackground)
            return;



        const width =
            this.settings.video.width;


        const height =
            this.settings.video.height;



        this.ctx.clearRect(
            0,
            0,
            width,
            height
        );



        const rect =
            containFit(
                video.videoWidth,
                video.videoHeight,
                width,
                height
            );



        this.ctx.drawImage(
            video,
            rect.x,
            rect.y,
            rect.width,
            rect.height
        );



        const current =
            this.ctx.getImageData(
                0,
                0,
                width,
                height
            );



        const pixels =
            current.data;



        const bgPixels =
            keying
            ?
            null
            :
            this.background.canvas
            .getContext("2d")
            .getImageData(
                0,
                0,
                width,
                height
            )
            .data;



        const keyColour =
            this.bodySettings.keyColour;



        const result =
            this.outputCtx.createImageData(
                width,
                height
            );



        const threshold =
            this.bodySettings.threshold;



        const videoFill =
            this.bodySettings.fill === "video";



        for(
            let i=0;
            i<pixels.length;
            i+=4
        ){


            const refR =
                keying ? keyColour.r : bgPixels[i];

            const refG =
                keying ? keyColour.g : bgPixels[i+1];

            const refB =
                keying ? keyColour.b : bgPixels[i+2];



            let difference =
                Math.abs(
                    pixels[i] -
                    refR
                )
                +
                Math.abs(
                    pixels[i+1] -
                    refG
                )
                +
                Math.abs(
                    pixels[i+2] -
                    refB
                );



            if(
                difference > threshold
            ){


                if(videoFill){

                    result.data[i] =
                        pixels[i];


                    result.data[i+1] =
                        pixels[i+1];


                    result.data[i+2] =
                        pixels[i+2];

                }
                else {

                    result.data[i] =
                        this.colour.r;


                    result.data[i+1] =
                        this.colour.g;


                    result.data[i+2] =
                        this.colour.b;

                }


                result.data[i+3] =
                    255;


            }


        }



        this.outputCtx.putImageData(
            result,
            0,
            0
        );


    }





    setColour(r,g,b){


        this.colour = {

            r:r,

            g:g,

            b:b

        };


        console.log(
            "Body colour:",
            r,
            g,
            b
        );


    }


}