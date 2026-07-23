/*
==================================================
DANCE TRACKER 5000
RENDER ENGINE
==================================================
*/


export class Renderer {



    constructor(options){


        this.settings = options.settings;
        this.camera = null;
        this.video = document.getElementById("camera");


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


            overlay:
            document.getElementById(
                "overlay-layer"
            )


        };


    Object.values(this.layers)
        .forEach(canvas=>{
                canvas.width=320;
                canvas.height=240;
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

            overlay:
            this.layers.overlay
            .getContext("2d")
        };
        
        this.running=false;
        this.lastTime=0;
        this.fps=0;
    }

    start(){


        this.running=true;


        requestAnimationFrame(

            this.loop.bind(this)

        );


    }








    loop(time){
        if(!this.running)
            return;

        let delta = time-this.lastTime;

        this.lastTime=time;

        if(delta>0) {
            this.fps= Math.round( 1000/delta );
        }

        this.clear();
        this.drawCamera();
        this.drawStatus();

        requestAnimationFrame(
            this.loop.bind(this)
        );
    }
    clear() {
        Object.values(
            this.contexts
        )

        .forEach(ctx=>{


            ctx.clearRect(

                0,

                0,

                ctx.canvas.width,

                ctx.canvas.height

            );


        });



    }








    drawStatus(){



        let status=

        document.querySelector(

            ".statusbar"

        );



        if(status){



            status.children[1]

            .innerText=

            "FPS: "+this.fps;



        }



    }



    drawCamera(){
        let ctx = this.contexts.effects;
        if( this.video.readyState >= 2 ) {
            ctx.drawImage(this.video, 0, 0, this.layers.effects.width, this.layers.effects.height);
        }
    }
}