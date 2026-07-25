/*
==================================================
DANCE TRACKER 5000
AMIGA CONCENTRIC RING GENERATOR
==================================================
*/


export class Rings {


    constructor(settings){


        this.settings = settings;


        this.canvas =
            document.getElementById(
                "rings-layer"
            );


        this.ctx =
            this.canvas.getContext(
                "2d"
            );


        this.time = 0;


        this.centres = [];


        this.hubPhase =
            Math.random() *
            Math.PI *
            2;


    }




    hub(){


        return {

            x:
            this.canvas.width / 2 +
            Math.sin(
                this.time * 0.4 +
                this.hubPhase
            )
            *
            40,


            y:
            this.canvas.height / 2 +
            Math.cos(
                this.time * 0.32 +
                this.hubPhase
            )
            *
            30

        };


    }




    constellationPosition(group, count){


        const hub =
            this.hub();


        let angle =
            (
                Math.PI * 2 /
                count
            )
            *
            group
            +
            this.time * 0.15;


        const distance =
            this.settings.amiga.rings
            .constellation.distance;


        return {

            x:
            hub.x +
            Math.cos(angle) *
            distance,


            y:
            hub.y +
            Math.sin(angle) *
            distance

        };


    }




    exitConstellation(){


        const count =
            this.settings.amiga.rings.count;


        for(
            let group = 0;
            group < count;
            group++
        ){


            let position =
                this.constellationPosition(
                    group,
                    count
                );


            if(!this.centres[group]){

                this.centres[group] = {

                    phase:
                    Math.random() *
                    Math.PI * 2,

                    speed:
                    0.5 +
                    Math.random()

                };

            }


            let centre =
                this.centres[group];


            centre.x =
                position.x -
                Math.sin(
                    this.time *
                    centre.speed +
                    centre.phase
                )
                *
                50;


            centre.y =
                position.y -
                Math.cos(
                    this.time *
                    centre.speed *
                    0.8 +
                    centre.phase
                )
                *
                40;


        }


    }





    ringColour(index){


        let colours =
            this.settings.amiga.rings.colours;



        return colours[
            index %
            colours.length
        ];


    }





    update(){


        this.time += 0.03;



        let count =
            this.settings.amiga.rings.count;



        while(
            this.centres.length < count
        ){


            this.centres.push({


                x:
                Math.random()
                *
                this.canvas.width,


                y:
                Math.random()
                *
                this.canvas.height,


                phase:
                Math.random()
                *
                Math.PI
                *
                2,


                speed:
                0.5 +
                Math.random()


            });


        }




        if(
            this.centres.length > count
        ){

            this.centres.length =
                count;

        }


    }





    draw(){


        const rings =
            this.settings.amiga.rings;



        if(!rings.enabled)
            return;



        let ctx =
            this.ctx;



        ctx.save();



        ctx.globalCompositeOperation =
            rings.blend ||
            "screen";



        let count =
            rings.count;



        const constellation =
            rings.constellation &&
            rings.constellation.enabled;



        for(
            let group = 0;
            group < count;
            group++
        ){



            let cx, cy;



            if(constellation){


                let position =
                    this.constellationPosition(
                        group,
                        count
                    );


                cx = position.x;

                cy = position.y;


            }
            else {


                let centre =
                    this.centres[group];



                if(!centre)
                    continue;




                cx =
                    centre.x +
                    Math.sin(
                        this.time *
                        centre.speed +
                        centre.phase
                    )
                    *
                    50;



                cy =
                    centre.y +
                    Math.cos(
                        this.time *
                        centre.speed *
                        0.8 +
                        centre.phase
                    )
                    *
                    40;


            }




            let colour =
                this.ringColour(
                    group
                );




            let ringsPerGroup =
                rings.ringsPerGroup ||
                8;



            const zoom =
                rings.size / 30;


            const unit =
                (
                    rings.spacing ||
                    14
                )
                *
                zoom;


            const innerRadius =
                unit * 0.3;



            for(
                let n = 0;
                n <= ringsPerGroup;
                n++
            ){



                let radius =
                    innerRadius +
                    n * unit;



                ctx.beginPath();



                ctx.arc(
                    cx,
                    cy,
                    radius,
                    0,
                    Math.PI * 2
                );



                ctx.strokeStyle =
                    colour;



                ctx.lineWidth =
                    rings.width;



                ctx.globalAlpha =
                    0.8;



                ctx.stroke();



            }


        }



        ctx.restore();



    }


}