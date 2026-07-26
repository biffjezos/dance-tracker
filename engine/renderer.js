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

        this.extraMaskLayers = [];



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
            name.indexOf("maskLayer:") === 0
        ){

            const layer =
                this.extraMaskLayers.find(
                    l=>l.id === name.slice(10)
                );

            return layer ? layer.canvas : null;

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




    /*
    Same ids as layerByName, but returns the whole settings bundle
    (canvas + maskedBy + visibilityMode + background) instead of just
    the canvas - resolveChainedLayer needs this to keep following a
    background chain (a layer's background can itself have a
    background), which layerByName's plain-canvas lookups don't
    support and shouldn't have to (MASKED BY only ever needs a shape,
    never the source's own background chain).
    */
    resolveEntryByMaskSourceId(id){


        if(id === "video")
            return {
                canvas:this.layers.effects,
                maskedBy:this.settings.video.maskedBy,
                visibilityMode:this.settings.video.visibilityMode,
                background:this.settings.video.background
            };

        if(id === "body")
            return {
                canvas:this.layers.body,
                maskedBy:this.settings.body.maskedBy,
                visibilityMode:this.settings.body.visibilityMode,
                background:this.settings.body.background
            };

        if(id === "rings")
            return {
                canvas:this.layers.rings,
                maskedBy:this.settings.amiga.rings.maskedBy,
                visibilityMode:this.settings.amiga.rings.visibilityMode,
                background:this.settings.amiga.rings.background
            };

        if(id === "ghost")
            return {
                canvas:this.layers.ghost,
                maskedBy:this.settings.amiga.ghost.maskedBy,
                visibilityMode:this.settings.amiga.ghost.visibilityMode,
                background:this.settings.amiga.ghost.background
            };

        if(id === "text")
            return {
                canvas:this.layers.text,
                maskedBy:this.settings.amiga.text.maskedBy,
                visibilityMode:this.settings.amiga.text.visibilityMode,
                background:this.settings.amiga.text.background
            };


        if(
            id &&
            id.indexOf("videoLayer:") === 0
        ){

            const layer =
                this.extraVideoLayers.find(
                    l=>l.id === id.slice(11)
                );

            return layer ? {
                canvas:layer.rawCanvas,
                maskedBy:layer.videoSettings.maskedBy,
                visibilityMode:layer.videoSettings.visibilityMode,
                background:layer.videoSettings.background
            } : null;

        }


        if(
            id &&
            id.indexOf("maskLayer:") === 0
        ){

            const layer =
                this.extraMaskLayers.find(
                    l=>l.id === id.slice(10)
                );

            return layer ? {
                canvas:layer.canvas,
                maskedBy:layer.bodySettings.maskedBy,
                visibilityMode:layer.bodySettings.visibilityMode,
                background:layer.bodySettings.background
            } : null;

        }


        if(
            id &&
            id.indexOf("ringsLayer:") === 0
        ){

            const layer =
                this.extraRingsLayers.find(
                    l=>l.id === id.slice(11)
                );

            return layer ? {
                canvas:layer.canvas,
                maskedBy:layer.ringsSettings.maskedBy,
                visibilityMode:layer.ringsSettings.visibilityMode,
                background:layer.ringsSettings.background
            } : null;

        }


        if(
            id &&
            id.indexOf("ghostLayer:") === 0
        ){

            const layer =
                this.extraGhostLayers.find(
                    l=>l.id === id.slice(11)
                );

            return layer ? {
                canvas:layer.canvas,
                maskedBy:layer.ghostSettings.maskedBy,
                visibilityMode:layer.ghostSettings.visibilityMode,
                background:layer.ghostSettings.background
            } : null;

        }


        if(
            id &&
            id.indexOf("textLayer:") === 0
        ){

            const layer =
                this.extraTextLayers.find(
                    l=>l.id === id.slice(10)
                );

            return layer ? {
                canvas:layer.canvas,
                maskedBy:layer.textSettings.maskedBy,
                visibilityMode:layer.textSettings.visibilityMode,
                background:layer.textSettings.background
            } : null;

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




    visualizeLayer(canvas, maskedBy, mode, background){


        const visualized =
            this.applyVisibilityMode(

                this.resolveMaskedLayer(
                    canvas,
                    maskedBy
                ),

                mode

            );


        if(
            !background ||
            background.source === "none"
        )
            return visualized;


        /*
        compositeWithBackground may recurse into resolveChainedLayer,
        which reuses these exact same shared scratch canvases
        (maskScratch/visibilityScratch) for whatever it's resolving
        further down the chain - snapshot our own result into a
        canvas nothing else touches before that happens, or the
        recursive call would stomp it before we get to use it.
        */
        return this.compositeWithBackground(
            this.snapshotCanvas(visualized),
            background,
            new Set()
        );

    }




    /*
    Copies a canvas's current pixels into a freshly-allocated canvas.
    Needed whenever a result from a shared scratch canvas
    (maskScratch/visibilityScratch/backgroundFillScratch) has to stay
    valid across a call that might reuse those same scratches -
    recursive chain resolution does exactly that.
    */
    snapshotCanvas(source){


        const copy =
            document.createElement(
                "canvas"
            );

        copy.width = source.width;

        copy.height = source.height;

        copy.getContext("2d").drawImage(
            source,
            0,
            0
        );

        return copy;

    }




    /*
    Follows a background chain to whichever real layer it points at,
    resolving THAT layer's own fully-composited appearance (its own
    mask+visibility+background, all the way down) rather than just
    its plain raw content - this is what actually makes chaining work
    (A's background is B, B's background is C, ...). visiting is the
    set of maskSourceIds already being resolved in this call stack;
    hitting one again means a cycle, and we fall back to that layer's
    own un-backgrounded appearance instead of recursing forever.
    */
    resolveChainedLayer(id, visiting){


        if(!id || id === "none")
            return null;


        const entry =
            this.resolveEntryByMaskSourceId(
                id
            );

        if(!entry)
            return null;


        if(visiting.has(id)){

            return this.snapshotCanvas(

                this.applyVisibilityMode(

                    this.resolveMaskedLayer(
                        entry.canvas,
                        entry.maskedBy
                    ),

                    entry.visibilityMode

                )

            );

        }


        visiting.add(id);


        const ownAppearance =
            this.snapshotCanvas(

                this.applyVisibilityMode(

                    this.resolveMaskedLayer(
                        entry.canvas,
                        entry.maskedBy
                    ),

                    entry.visibilityMode

                )

            );


        let result =
            ownAppearance;


        if(
            entry.background &&
            entry.background.source !== "none"
        ){

            result =
                this.compositeWithBackground(
                    ownAppearance,
                    entry.background,
                    visiting
                );

        }


        visiting.delete(id);


        return result;

    }




    /*
    Resolves a background spec to a fillable canvas: another layer's
    fully-chained appearance if it points at one (falling back to the
    flat colour if that source doesn't resolve or the chain cycles
    back here), otherwise the flat colour itself.
    */
    resolveBackgroundFill(background, visiting){


        if(background.source !== "colour"){

            const resolved =
                this.resolveChainedLayer(
                    background.source,
                    visiting
                );

            if(resolved)
                return resolved;

        }


        if(!this.backgroundFillScratch){

            this.backgroundFillScratch =
                document.createElement(
                    "canvas"
                );

        }


        this.backgroundFillScratch.width =
            this.settings.video.width;

        this.backgroundFillScratch.height =
            this.settings.video.height;


        const ctx =
            this.backgroundFillScratch.getContext(
                "2d"
            );


        ctx.fillStyle =
            "rgb("
            +
            background.colour.r
            +
            ","
            +
            background.colour.g
            +
            ","
            +
            background.colour.b
            +
            ")";


        ctx.fillRect(

            0,

            0,

            this.backgroundFillScratch.width,

            this.backgroundFillScratch.height

        );


        return this.backgroundFillScratch;

    }




    /*
    Output is a freshly-allocated canvas, not a shared scratch - this
    can be called re-entrantly (resolveBackgroundFill below may
    recurse back into this same method for a deeper link in the
    chain), and a shared scratch would get overwritten by the inner
    call before the outer call finishes drawing into it.
    */
    compositeWithBackground(contentCanvas, background, visiting){


        const width =
            contentCanvas.width;

        const height =
            contentCanvas.height;


        const result =
            document.createElement(
                "canvas"
            );

        result.width = width;

        result.height = height;


        const ctx =
            result.getContext(
                "2d"
            );


        ctx.drawImage(
            this.resolveBackgroundFill(
                background,
                visiting
            ),
            0,
            0
        );


        ctx.drawImage(
            contentCanvas,
            0,
            0
        );


        return result;

    }




    /*
    A layer's own kind has a default way it sits on the master -
    source-over for video/body/text, screen for rings/ghost (their
    glow). That default is only meant for the layer's OWN content
    though. Once a BACKGROUND fill is added, visualizeLayer/
    compositeWithBackground has already flattened that fill together
    with the content into one canvas - if the kind's default then
    still applied to the whole thing, a rings layer's flat green
    background would screen-blend straight into whatever's already on
    the master (e.g. another layer's own background colour beneath
    it), mixing two unrelated flat fills into a third colour neither
    one chose. Once a background is active, its own blendMode decides
    how the whole flattened result sits on the master instead.
    */
    resolveCompositeOperation(defaultOperation, background){

        if(
            !background ||
            background.source === "none"
        )
            return defaultOperation;

        if(background.blendMode === "overlay")
            return "overlay";

        if(background.blendMode === "screen")
            return "screen";

        return "source-over";

    }




    /*
    Every id any real entry could currently have - used to ask each
    one "is anything using you as its background right now?" so
    computeConsumedIds can answer that without needing app.js's
    registries (which renderer.js doesn't have access to).
    */
    allComposableIds(){

        const ids = [
            "video", "body", "rings", "ghost", "text"
        ];

        this.extraVideoLayers.forEach(layer=>{

            ids.push(
                "videoLayer:" + layer.id
            );

        });

        this.extraMaskLayers.forEach(layer=>{

            ids.push("maskLayer:" + layer.id);

        });

        this.extraRingsLayers.forEach(layer=>{

            ids.push("ringsLayer:" + layer.id);

        });

        this.extraGhostLayers.forEach(layer=>{

            ids.push("ghostLayer:" + layer.id);

        });

        this.extraTextLayers.forEach(layer=>{

            ids.push("textLayer:" + layer.id);

        });

        return ids;

    }




    /*
    "I want layers only be in the background" - once node X is wired
    as node Y's background, X should not also independently draw
    itself into the master (that's the whole point of a node-based
    background link: X's output routes into Y, it doesn't also fan
    out to the final compositor unless nothing else uses it). Without
    this, X's own composited appearance (correctly showing Y where X
    is transparent) gets drawn, then X draws AGAIN on its own at its
    normal stack position, covering the composite that was just made.

    Only background links suppress independent drawing - MASKED BY
    doesn't (a mask being used as someone else's shape is still a
    perfectly good thing to look at on its own, e.g. in MASK WHITE
    mode). A cycle (A's background is B, B's background is A) simply
    means both ask to suppress each other - resolveChainedLayer
    already falls back safely when it hits that same cycle while
    resolving actual pixel content, so this can't produce a case
    where nothing draws at all only where content briefly looks odd.
    */
    computeConsumedIds(){

        const consumed = new Set();

        this.allComposableIds().forEach(id=>{

            const entry =
                this.resolveEntryByMaskSourceId(id);

            if(!entry)
                return;

            if(entry.visibilityMode === "off")
                return;

            if(!entry.background)
                return;

            if(
                entry.background.source === "none" ||
                entry.background.source === "colour"
            )
                return;

            consumed.add(
                entry.background.source
            );

        });

        return consumed;

    }




    compose(){


        let ctx =
            this.contexts.master;


        const consumedIds =
            this.computeConsumedIds();



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
            this.settings.video.visibilityMode !== "off" &&
            !consumedIds.has("video")
        ){

            ctx.globalCompositeOperation =
                this.resolveCompositeOperation(
                    "source-over",
                    this.settings.video.background
                );

            ctx.drawImage(

                this.visualizeLayer(
                    this.layers.effects,
                    this.settings.video.maskedBy,
                    this.settings.video.visibilityMode,
                    this.settings.video.background
                ),

                0,

                0

            );

        }




        if(
            this.settings.layers.body &&
            this.settings.body.visibilityMode !== "off" &&
            !consumedIds.has("body")
        ){

            ctx.globalCompositeOperation =
                this.resolveCompositeOperation(
                    "source-over",
                    this.settings.body.background
                );

            ctx.drawImage(

                this.visualizeLayer(
                    this.layers.body,
                    this.settings.body.maskedBy,
                    this.settings.body.visibilityMode,
                    this.settings.body.background
                ),

                0,

                0

            );

        }


        this.composeExtraMaskLayers(consumedIds);




        if(
            this.settings.amiga.rings.visibilityMode !== "off" &&
            !consumedIds.has("rings")
        ){

            ctx.globalCompositeOperation =
                this.resolveCompositeOperation(
                    "screen",
                    this.settings.amiga.rings.background
                );

            ctx.drawImage(

                this.visualizeLayer(
                    this.layers.rings,
                    this.settings.amiga.rings.maskedBy,
                    this.settings.amiga.rings.visibilityMode,
                    this.settings.amiga.rings.background
                ),

                0,

                0

            );

        }


        this.composeExtraRingsLayers(consumedIds);



        if(
            this.settings.amiga.ghost.visibilityMode !== "off" &&
            !consumedIds.has("ghost")
        ){

            ctx.globalCompositeOperation =
                this.resolveCompositeOperation(
                    "screen",
                    this.settings.amiga.ghost.background
                );

            ctx.drawImage(

                this.visualizeLayer(
                    this.layers.ghost,
                    this.settings.amiga.ghost.maskedBy,
                    this.settings.amiga.ghost.visibilityMode,
                    this.settings.amiga.ghost.background
                ),

                0,

                0

            );

        }


        this.composeExtraGhostLayers(consumedIds);



        if(
            this.settings.amiga.text.visibilityMode !== "off" &&
            !consumedIds.has("text")
        ){

            ctx.globalCompositeOperation =
                this.resolveCompositeOperation(
                    "source-over",
                    this.settings.amiga.text.background
                );

            ctx.drawImage(

                this.visualizeLayer(
                    this.layers.text,
                    this.settings.amiga.text.maskedBy,
                    this.settings.amiga.text.visibilityMode,
                    this.settings.amiga.text.background
                ),

                0,

                0

            );

        }


        this.composeExtraTextLayers(consumedIds);



        this.composeExtraVideoLayers(consumedIds);



        ctx.restore();


    }




    composeExtraVideoLayers(consumedIds){


        let ctx =
            this.contexts.master;


        this.extraVideoLayers.forEach(layer=>{


            if(
                layer.videoSettings.enabled &&
                layer.videoSettings.visibilityMode !== "off" &&
                !consumedIds.has("videoLayer:" + layer.id)
            ){

                ctx.globalCompositeOperation =
                    this.resolveCompositeOperation(
                        "source-over",
                        layer.videoSettings.background
                    );

                ctx.drawImage(

                    this.visualizeLayer(
                        layer.rawCanvas,
                        layer.videoSettings.maskedBy,
                        layer.videoSettings.visibilityMode,
                        layer.videoSettings.background
                    ),

                    0,

                    0

                );

            }


        });


    }




    composeExtraMaskLayers(consumedIds){


        let ctx =
            this.contexts.master;


        this.extraMaskLayers.forEach(layer=>{


            if(
                layer.bodySettings.enabled &&
                layer.bodySettings.visibilityMode !== "off" &&
                !consumedIds.has("maskLayer:" + layer.id)
            ){

                ctx.globalCompositeOperation =
                    this.resolveCompositeOperation(
                        "source-over",
                        layer.bodySettings.background
                    );

                ctx.drawImage(

                    this.visualizeLayer(
                        layer.canvas,
                        layer.bodySettings.maskedBy,
                        layer.bodySettings.visibilityMode,
                        layer.bodySettings.background
                    ),

                    0,

                    0

                );

            }


        });


    }




    composeExtraRingsLayers(consumedIds){


        let ctx =
            this.contexts.master;


        this.extraRingsLayers.forEach(layer=>{


            if(
                layer.ringsSettings.visibilityMode !== "off" &&
                !consumedIds.has("ringsLayer:" + layer.id)
            ){

                ctx.globalCompositeOperation =
                    this.resolveCompositeOperation(
                        "screen",
                        layer.ringsSettings.background
                    );

                ctx.drawImage(

                    this.visualizeLayer(
                        layer.canvas,
                        layer.ringsSettings.maskedBy,
                        layer.ringsSettings.visibilityMode,
                        layer.ringsSettings.background
                    ),

                    0,

                    0

                );

            }


        });


    }




    composeExtraGhostLayers(consumedIds){


        let ctx =
            this.contexts.master;


        this.extraGhostLayers.forEach(layer=>{


            if(
                layer.ghostSettings.visibilityMode !== "off" &&
                !consumedIds.has("ghostLayer:" + layer.id)
            ){

                ctx.globalCompositeOperation =
                    this.resolveCompositeOperation(
                        "screen",
                        layer.ghostSettings.background
                    );

                ctx.drawImage(

                    this.visualizeLayer(
                        layer.canvas,
                        layer.ghostSettings.maskedBy,
                        layer.ghostSettings.visibilityMode,
                        layer.ghostSettings.background
                    ),

                    0,

                    0

                );

            }


        });


    }




    composeExtraTextLayers(consumedIds){


        let ctx =
            this.contexts.master;


        this.extraTextLayers.forEach(layer=>{


            if(
                layer.textSettings.visibilityMode !== "off" &&
                !consumedIds.has("textLayer:" + layer.id)
            ){

                ctx.globalCompositeOperation =
                    this.resolveCompositeOperation(
                        "source-over",
                        layer.textSettings.background
                    );

                ctx.drawImage(

                    this.visualizeLayer(
                        layer.canvas,
                        layer.textSettings.maskedBy,
                        layer.textSettings.visibilityMode,
                        layer.textSettings.background
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