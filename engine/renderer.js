/*
==================================================
DANCE TRACKER 5000
RENDER ENGINE
==================================================
*/


import { containFit } from "./fit.js";


export class Renderer {


    constructor(options){


        this.settings =
            options.settings;


        this.video =
            document.getElementById(
                "camera"
            );



        this.layers = {


            background:
            document.getElementById(
                "background-layer"
            ),


            effects:
            document.getElementById(
                "effect-layer"
            ),


            body:
            document.getElementById(
                "body-layer"
            ),


            rings:
            document.getElementById(
                "rings-layer"
            ),


            ghost:
            document.getElementById(
                "ghost-layer"
            ),


            text:
            document.getElementById(
                "text-layer"
            ),


            master:
            document.getElementById(
                "master-layer"
            )


        };



        Object.values(this.layers)
        .forEach(canvas=>{


            canvas.width =
                this.settings.video.width;

            canvas.height =
                this.settings.video.height;


        });



        this.contexts = {


            background:
            this.layers.background
            .getContext("2d"),



            effects:
            this.layers.effects
            .getContext("2d"),



            body:
            this.layers.body
            .getContext("2d"),



            rings:
            this.layers.rings
            .getContext("2d"),



            ghost:
            this.layers.ghost
            .getContext("2d"),



            text:
            this.layers.text
            .getContext("2d"),



            master:
            this.layers.master
            .getContext("2d")


        };



        this.running = false;

        this.lastTime = 0;

        this.fps = 0;


    }





    start(){


        this.running = true;


        requestAnimationFrame(
            this.loop.bind(this)
        );


    }




    resize(){


        Object.values(this.layers)
        .forEach(canvas=>{


            canvas.width =
                this.settings.video.width;

            canvas.height =
                this.settings.video.height;


        });


    }






    loop(time){


        if(!this.running)
            return;



        let delta =
            time - this.lastTime;



        this.lastTime = time;



        if(delta > 0){

            this.fps =
                Math.round(
                    1000 / delta
                );

        }



        this.clearMaster();

        this.drawBackground();

        this.drawCamera();

        this.compose();

        this.drawStatus();



        requestAnimationFrame(
            this.loop.bind(this)
        );


    }





    clearMaster(){


        this.contexts.master.clearRect(

            0,

            0,

            this.settings.video.width,

            this.settings.video.height

        );



        this.contexts.effects.clearRect(

            0,

            0,

            this.settings.video.width,

            this.settings.video.height

        );


    }






    drawBackground(){


        this.contexts.background.fillStyle =
            this.settings.video.backgroundColour;


        this.contexts.background.fillRect(

            0,

            0,

            this.settings.video.width,

            this.settings.video.height

        );


    }




    drawCamera(){


        if(
            !this.settings.video.enabled
        )
            return;



        if(
            this.video.readyState < 2
        )
            return;



        const rect =
            containFit(
                this.video.videoWidth,
                this.video.videoHeight,
                this.settings.video.width,
                this.settings.video.height
            );



        this.contexts.effects.drawImage(

            this.video,

            rect.x,

            rect.y,

            rect.width,

            rect.height

        );


    }






    layerByName(name){


        if(name === "background")
            return this.layers.background;

        if(name === "video")
            return this.layers.effects;

        if(name === "body")
            return this.layers.body;

        if(name === "rings")
            return this.layers.rings;

        if(name === "ghost")
            return this.layers.ghost;

        if(name === "text")
            return this.layers.text;


        return null;

    }




    maskChannelValue(pixels, i, channel){


        if(channel === "red")
            return pixels[i];

        if(channel === "green")
            return pixels[i+1];

        if(channel === "blue")
            return pixels[i+2];


        return pixels[i+3];

    }




    applyMask(contentCanvas, sourceCanvas, channel){


        const width =
            contentCanvas.width;

        const height =
            contentCanvas.height;


        if(!this.maskScratch){

            this.maskScratch =
                document.createElement(
                    "canvas"
                );

        }


        this.maskScratch.width = width;

        this.maskScratch.height = height;


        const scratchCtx =
            this.maskScratch.getContext(
                "2d"
            );


        scratchCtx.clearRect(
            0,
            0,
            width,
            height
        );

        scratchCtx.drawImage(
            contentCanvas,
            0,
            0
        );


        const content =
            scratchCtx.getImageData(
                0,
                0,
                width,
                height
            );


        const source =
            sourceCanvas
            .getContext("2d")
            .getImageData(
                0,
                0,
                width,
                height
            );


        const contentPixels =
            content.data;

        const sourcePixels =
            source.data;


        for(
            let i=0;
            i<contentPixels.length;
            i+=4
        ){


            const maskValue =
                this.maskChannelValue(
                    sourcePixels,
                    i,
                    channel
                );


            contentPixels[i+3] =
                Math.round(
                    contentPixels[i+3] *
                    (maskValue / 255)
                );


        }


        scratchCtx.putImageData(
            content,
            0,
            0
        );


        return this.maskScratch;

    }




    resolveMaskedLayer(canvas, maskedBy){


        if(
            !maskedBy ||
            maskedBy.source === "none"
        )
            return canvas;


        const sourceCanvas =
            this.layerByName(
                maskedBy.source
            );


        if(!sourceCanvas)
            return canvas;


        return this.applyMask(
            canvas,
            sourceCanvas,
            maskedBy.channel
        );

    }




    compose(){


        let ctx =
            this.contexts.master;



        ctx.save();



        ctx.globalCompositeOperation =
            "source-over";



        ctx.drawImage(

            this.layers.background,

            0,

            0

        );



        if(
            this.settings.video.enabled &&
            this.settings.video.visible
        ){

            ctx.drawImage(

                this.resolveMaskedLayer(
                    this.layers.effects,
                    this.settings.video.maskedBy
                ),

                0,

                0

            );

        }




        if(
            this.settings.layers.body &&
            this.settings.body.visible
        ){

            ctx.drawImage(

                this.resolveMaskedLayer(
                    this.layers.body,
                    this.settings.body.maskedBy
                ),

                0,

                0

            );

        }




        ctx.globalCompositeOperation =
            "screen";



        if(
            this.settings.amiga.rings.visible
        ){

            ctx.drawImage(

                this.resolveMaskedLayer(
                    this.layers.rings,
                    this.settings.amiga.rings.maskedBy
                ),

                0,

                0

            );

        }



        ctx.drawImage(

            this.layers.ghost,

            0,

            0

        );



        ctx.globalCompositeOperation =
            "source-over";



        ctx.drawImage(

            this.resolveMaskedLayer(
                this.layers.text,
                this.settings.amiga.text.maskedBy
            ),

            0,

            0

        );



        ctx.restore();


    }






    drawStatus(){


        let status =
            document.querySelector(
                ".statusbar"
            );



        if(status){


            status.children[1].innerText =
                "FPS: " + this.fps;


        }


    }


}