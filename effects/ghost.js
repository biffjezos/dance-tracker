/*
==================================================
DANCE TRACKER 5000
BODY GHOST TRAIL EFFECT
==================================================
*/

export class Ghost {

    constructor(settings){

        this.settings = settings;

        this.canvas =
            document.getElementById(
                "effect-layer"
            );

        this.ctx =
            this.canvas.getContext("2d");


        this.history = [];

    }



    update(){

        const ghost =
            this.settings.amiga.ghost;


        if(!ghost.enabled){

            this.history = [];

            return;

        }


        const body =
            document.getElementById(
                "body-layer"
            );


        let copy =
            document.createElement(
                "canvas"
            );


        copy.width =
            body.width;

        copy.height =
            body.height;


        let ctx =
            copy.getContext(
                "2d"
            );


        ctx.drawImage(
            body,
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



        for(
            let i=this.history.length-1;
            i>=0;
            i--
        ){

            let alpha =
                ghost.alpha *
                (
                    1 -
                    i /
                    (this.history.length + 1)
                );


            ctx.globalAlpha =
                alpha;


            ctx.drawImage(
                this.history[i],
                0,
                0
            );

        }


        ctx.restore();

    }

}
