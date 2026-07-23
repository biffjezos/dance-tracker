/*
==================================================
DANCE TRACKER 5000
BODY GHOST TRAIL EFFECT
==================================================
*/

export class Ghost {

    constructor(settings, palette){

        this.settings = settings;
        this.palette = palette;
        this.canvas =
            document.getElementById(
                "overlay-layer"
            );

        this.ctx =
            this.canvas.getContext("2d");


        this.body =
            document.getElementById(
                "body-layer"
            );


        this.history = [];

    }

    getColour(){

        let colour =
            this.palette.get().ghost;


        let hex =
            colour.replace("#","");


        return {

            r:parseInt(
                hex.substring(0,2),
                16
            ),

            g:parseInt(
                hex.substring(2,4),
                16
            ),

            b:parseInt(
                hex.substring(4,6),
                16
            )

        };

    }

    update(){

        const ghost =
            this.settings.amiga.ghost;


        if(!ghost.enabled){

            this.history = [];

            return;

        }


        let copy =
            document.createElement(
                "canvas"
            );


        copy.width =
            this.body.width;

        copy.height =
            this.body.height;


        let ctx =
            copy.getContext("2d");


        ctx.drawImage(
            this.body,
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
            this.settings.amiga.ghost;


        if(!ghost.enabled)
            return;


        let ctx =
            this.ctx;


        ctx.save();


        ctx.globalCompositeOperation =
            "screen";

        let colour = this.getColour();
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
