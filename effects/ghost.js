/*
==================================================
DANCE TRACKER 5000
BODY GHOST TRAIL EFFECT
==================================================
*/


let nextGhostNumber = 2;


export class Ghost {

    constructor(settings, renderer, options){

        options = options || {};

        this.settings = settings;
        this.renderer = renderer;


        const isAdded =
            !!options.ghostSettings;


        this.ghostSettings =
            options.ghostSettings ||
            settings.amiga.ghost;


        this.number =
            isAdded
            ?
            nextGhostNumber++
            :
            1;


        this.id =
            isAdded
            ?
            ("ghost-layer-" + this.number)
            :
            "ghost";


        this.name =
            "GHOST " + this.number;


        this.canvas =
            options.outputCanvas ||
            document.getElementById(
                "ghost-layer"
            );

        this.ctx =
            this.canvas.getContext("2d");


        this.history = [];

        this.lastCapture = 0;

        this.elapsed = 0;

    }



    resize(){

        this.canvas.width =
            this.settings.video.width;

        this.canvas.height =
            this.settings.video.height;

    }



    update(){

        const ghost =
            this.ghostSettings;


        if(!ghost.enabled){

            this.history = [];

            this.lastCapture = 0;

            this.elapsed = 0;

            return;

        }


        const source =
            this.renderer.layerByName(
                ghost.applyToMask
            );


        if(!source)
            return;


        let now =
            performance.now();


        if(!this.lastCapture)
            this.lastCapture = now;


        this.elapsed +=
            now - this.lastCapture;


        this.lastCapture = now;


        if(this.elapsed < ghost.delay)
            return;


        this.elapsed = 0;


        let copy =
            document.createElement(
                "canvas"
            );


        copy.width =
            source.width;

        copy.height =
            source.height;


        let ctx =
            copy.getContext("2d");


        ctx.drawImage(
            source,
            0,
            0
        );


        this.history.unshift(
            copy
        );


        while(
            this.history.length >
            ghost.count
        ){

            this.history.pop();

        }

    }



    draw(){

        const ghost =
            this.ghostSettings;


        if(!ghost.enabled)
            return;


        let ctx =
            this.ctx;


        ctx.save();


        ctx.globalCompositeOperation =
            "screen";

        for(
            let i=0;
            i<this.history.length;
            i++
        ){

            ctx.globalAlpha =
                ghost.alpha *
                (
                    1 -
                    i /
                    this.history.length
                );
            ctx.globalCompositeOperation = "screen";

            ctx.drawImage(
                this.history[i],
                0,
                0
            );

        }


        ctx.restore();

    }

}
