/*
==================================================
DANCE TRACKER 5000
RENDER ENGINE
==================================================
*/


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


            overlay:
            document.getElementById(
                "overlay-layer"
            ),


            master:
            document.getElementById(
                "master-layer"
            )


        };



        Object.values(this.layers)
        .forEach(canvas=>{


            canvas.width = 320;

            canvas.height = 240;


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

            320,

            240

        );



        this.contexts.effects.clearRect(

            0,

            0,

            320,

            240

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



        this.contexts.effects.drawImage(

            this.video,

            0,

            0,

            320,

            240

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
            this.settings.video.enabled
        ){

            ctx.drawImage(

                this.layers.effects,

                0,

                0

            );

        }




        if(
            this.settings.layers.body
        ){

            ctx.drawImage(

                this.layers.body,

                0,

                0

            );

        }




        ctx.globalCompositeOperation =
            "screen";



        ctx.drawImage(

            this.layers.overlay,

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