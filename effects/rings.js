/*
==================================================
DANCE TRACKER 5000
AMIGA CONCENTRIC RING GENERATOR
==================================================
*/

export class Rings {

    constructor(settings, palette){

        this.settings = settings;

        this.palette = palette;
        this.palette.get().rings;
        this.canvas =
            document.getElementById(
                "overlay-layer"
            );

        this.ctx =
            this.canvas.getContext("2d");

        this.time = 0;

        this.centres = [];

    }
    paletteColour(index) {
        let colours =  this.palette.get().rings;
        return colours[  index % colours.length ];
    }

    hexToRGB(hex){

        hex =
            hex.replace("#","");


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

        this.time += 0.03;


        let count =
            this.settings.amiga.rings.count;


        while(this.centres.length < count){

            this.centres.push({

                x: Math.random() * this.canvas.width,

                y: Math.random() * this.canvas.height,

                phase: Math.random() * Math.PI * 2,

                speed:
                    0.5 +
                    Math.random()

            });

        }


        if(this.centres.length > count){

            this.centres.length = count;

        }

    }




    draw(){

        const rings =
            this.settings.amiga.rings;


        if(!rings.enabled)
            return;


        let ctx = this.ctx;


        ctx.save();


        ctx.globalCompositeOperation =
            rings.blend || "screen";


        let count =
            rings.count;


        for(
            let group = 0;
            group < count;
            group++
        ){


            let centre =
                this.centres[group];


            if(!centre)
                continue;



            let cx =
                centre.x +
                Math.sin(
                    this.time *
                    centre.speed +
                    centre.phase
                ) * 50;



            let cy =
                centre.y +
                Math.cos(
                    this.time *
                    centre.speed * 0.8 +
                    centre.phase
                ) * 40;



            let colour = this.paletteColour(group);



            let ringsPerGroup =
                rings.ringsPerGroup ||
                8;



            for(
                let r = 0;
                r < ringsPerGroup;
                r++
            ){


                let radius =
                    rings.size +
                    r *
                    (rings.spacing || 14);



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
