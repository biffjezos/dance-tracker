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


        this.extraVideoLayers = [];

        this.extraRingsLayers = [];

        this.extraGhostLayers = [];

        this.extraTextLayers = [];



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


        if(
            name &&
            name.indexOf("videoLayer:") === 0
        ){

            const layer =
                this.extraVideoLayers.find(
                    l=>l.id === name.slice(11)
                );

            return layer ? layer.rawCanvas : null;

        }


        if(
            name &&
            name.indexOf("bodyLayer:") === 0
        ){

            const layer =
                this.extraVideoLayers.find(
                    l=>l.id === name.slice(10)
                );

            return layer ? layer.bodyCanvas : null;

        }


        if(
            name &&
            name.indexOf("ringsLayer:") === 0
        ){

            const layer =
                this.extraRingsLayers.find(
                    l=>l.id === name.slice(11)
                );

            return layer ? layer.canvas : null;

        }


        if(
            name &&
            name.indexOf("ghostLayer:") === 0
        ){

            const layer =
                this.extraGhostLayers.find(
                    l=>l.id === name.slice(11)
                );

            return layer ? layer.canvas : null;

        }


        if(
            name &&
            name.indexOf("textLayer:") === 0
        ){

            const layer =
                this.extraTextLayers.find(
                    l=>l.id === name.slice(10)
                );

            return layer ? layer.canvas : null;

        }


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




    /*
    Visibility mode is a per-layer visual transform applied after
    masking, on top of the enabled/off gate the caller already checked.
    "on" passes the canvas through untouched. "maskWhite" flattens the
    shape to solid white RGB (keeping the existing alpha as the shape) -
    both a clean visual silhouette and, by construction, a correct mask
    source for any channel a downstream MASKED BY picks. "alpha"
    visualizes the raw alpha channel as an opaque greyscale image, since
    alpha alone isn't otherwise visible on screen.
    */
    applyVisibilityMode(canvas, mode){


        if(!mode || mode === "on")
            return canvas;


        const width =
            canvas.width;

        const height =
            canvas.height;


        if(!this.visibilityScratch){

            this.visibilityScratch =
                document.createElement(
                    "canvas"
                );

        }


        this.visibilityScratch.width = width;

        this.visibilityScratch.height = height;


        const scratchCtx =
            this.visibilityScratch.getContext(
                "2d"
            );


        scratchCtx.clearRect(
            0,
            0,
            width,
            height
        );

        scratchCtx.drawImage(
            canvas,
            0,
            0
        );


        const frame =
            scratchCtx.getImageData(
                0,
                0,
                width,
                height
            );


        const pixels =
            frame.data;


        if(mode === "maskWhite"){

            for(
                let i=0;
                i<pixels.length;
                i+=4
            ){

                pixels[i] = 255;

                pixels[i+1] = 255;

                pixels[i+2] = 255;

            }

        }
        else if(mode === "alpha"){

            for(
                let i=0;
                i<pixels.length;
                i+=4
            ){

                const a =
                    pixels[i+3];

                pixels[i] = a;

                pixels[i+1] = a;

                pixels[i+2] = a;

                pixels[i+3] = 255;

            }

        }


        scratchCtx.putImageData(
            frame,
            0,
            0
        );


        return this.visibilityScratch;

    }




    visualizeLayer(canvas, maskedBy, mode){


        return this.applyVisibilityMode(

            this.resolveMaskedLayer(
                canvas,
                maskedBy
            ),

            mode

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
            this.settings.video.visibilityMode !== "off"
        ){

            ctx.drawImage(

                this.visualizeLayer(
                    this.layers.effects,
                    this.settings.video.maskedBy,
                    this.settings.video.visibilityMode
                ),

                0,

                0

            );

        }




        if(
            this.settings.layers.body &&
            this.settings.body.visibilityMode !== "off"
        ){

            ctx.drawImage(

                this.visualizeLayer(
                    this.layers.body,
                    this.settings.body.maskedBy,
                    this.settings.body.visibilityMode
                ),

                0,

                0

            );

        }




        ctx.globalCompositeOperation =
            "screen";



        if(
            this.settings.amiga.rings.visibilityMode !== "off"
        ){

            ctx.drawImage(

                this.visualizeLayer(
                    this.layers.rings,
                    this.settings.amiga.rings.maskedBy,
                    this.settings.amiga.rings.visibilityMode
                ),

                0,

                0

            );

        }


        this.composeExtraRingsLayers();



        if(
            this.settings.amiga.ghost.visibilityMode !== "off"
        ){

            ctx.drawImage(

                this.visualizeLayer(
                    this.layers.ghost,
                    this.settings.amiga.ghost.maskedBy,
                    this.settings.amiga.ghost.visibilityMode
                ),

                0,

                0

            );

        }


        this.composeExtraGhostLayers();



        ctx.globalCompositeOperation =
            "source-over";



        if(
            this.settings.amiga.text.visibilityMode !== "off"
        ){

            ctx.drawImage(

                this.visualizeLayer(
                    this.layers.text,
                    this.settings.amiga.text.maskedBy,
                    this.settings.amiga.text.visibilityMode
                ),

                0,

                0

            );

        }


        this.composeExtraTextLayers();



        this.composeExtraVideoLayers();



        ctx.restore();


    }




    composeExtraVideoLayers(){


        let ctx =
            this.contexts.master;


        this.extraVideoLayers.forEach(layer=>{


            ctx.globalCompositeOperation =
                "source-over";


            if(
                layer.videoSettings.enabled &&
                layer.videoSettings.visibilityMode !== "off"
            ){

                ctx.drawImage(

                    this.visualizeLayer(
                        layer.rawCanvas,
                        layer.videoSettings.maskedBy,
                        layer.videoSettings.visibilityMode
                    ),

                    0,

                    0

                );

            }



            if(
                layer.bodySettings.enabled &&
                layer.bodySettings.visibilityMode !== "off"
            ){

                ctx.drawImage(

                    this.visualizeLayer(
                        layer.bodyCanvas,
                        layer.bodySettings.maskedBy,
                        layer.bodySettings.visibilityMode
                    ),

                    0,

                    0

                );

            }


        });


    }




    composeExtraRingsLayers(){


        let ctx =
            this.contexts.master;


        this.extraRingsLayers.forEach(layer=>{


            if(layer.ringsSettings.visibilityMode !== "off"){

                ctx.drawImage(

                    this.visualizeLayer(
                        layer.canvas,
                        layer.ringsSettings.maskedBy,
                        layer.ringsSettings.visibilityMode
                    ),

                    0,

                    0

                );

            }


        });


    }




    composeExtraGhostLayers(){


        let ctx =
            this.contexts.master;


        this.extraGhostLayers.forEach(layer=>{


            if(layer.ghostSettings.visibilityMode !== "off"){

                ctx.drawImage(

                    this.visualizeLayer(
                        layer.canvas,
                        layer.ghostSettings.maskedBy,
                        layer.ghostSettings.visibilityMode
                    ),

                    0,

                    0

                );

            }


        });


    }




    composeExtraTextLayers(){


        let ctx =
            this.contexts.master;


        this.extraTextLayers.forEach(layer=>{


            if(layer.textSettings.visibilityMode !== "off"){

                ctx.drawImage(

                    this.visualizeLayer(
                        layer.canvas,
                        layer.textSettings.maskedBy,
                        layer.textSettings.visibilityMode
                    ),

                    0,

                    0

                );

            }


        });


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