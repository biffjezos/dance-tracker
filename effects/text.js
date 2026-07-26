/*
==================================================
DANCE TRACKER 5000
BOLD OVERLAY TEXT
==================================================
*/


let nextTextNumber = 1;


export class Text {


    constructor(settings, options){


        options = options || {};

        this.settings = settings;


        const isAdded =
            !!options.textSettings;


        this.textSettings =
            options.textSettings ||
            settings.amiga.text;


        this.number =
            isAdded
            ?
            nextTextNumber++
            :
            1;


        this.id =
            isAdded
            ?
            ("text-layer-" + this.number)
            :
            "text";


        this.name =
            "TEXT " + this.number;


        this.canvas =
            options.outputCanvas ||
            document.getElementById(
                "text-layer"
            );


        this.ctx =
            this.canvas.getContext(
                "2d"
            );


    }



    resize(){

        this.canvas.width =
            this.settings.video.width;

        this.canvas.height =
            this.settings.video.height;

    }




    wrapLines(ctx, rawText, maxWidth){


        const paragraphs =
            rawText.split(
                /\\n|\n/
            );


        const lines = [];


        paragraphs.forEach(paragraph=>{


            const words =
                paragraph
                .split(" ")
                .filter(word=>
                    word.length > 0
                );


            if(words.length === 0){

                lines.push("");

                return;

            }


            let currentLine =
                words[0];


            for(
                let i = 1;
                i < words.length;
                i++
            ){


                let testLine =
                    currentLine +
                    " " +
                    words[i];


                if(
                    ctx.measureText(
                        testLine
                    ).width
                    >
                    maxWidth
                ){

                    lines.push(
                        currentLine
                    );

                    currentLine =
                        words[i];

                }
                else {

                    currentLine =
                        testLine;

                }


            }


            lines.push(
                currentLine
            );


        });


        return lines;


    }




    draw(){


        const text =
            this.textSettings;


        const ctx =
            this.ctx;


        ctx.clearRect(

            0,

            0,

            this.canvas.width,

            this.canvas.height

        );


        const content =
            text.content.trim();


        if(!content)
            return;


        ctx.save();


        ctx.font =
            "bold " +
            text.size +
            "px Arial, sans-serif";


        ctx.fillStyle =
            text.colour;


        ctx.textAlign =
            "center";


        ctx.textBaseline =
            "middle";


        const maxWidth =
            this.canvas.width - 20;


        const lines =
            this.wrapLines(
                ctx,
                text.content,
                maxWidth
            );


        const lineHeight =
            text.size * 1.15;


        const totalHeight =
            lines.length *
            lineHeight;


        let y =
            (
                this.canvas.height -
                totalHeight
            )
            /
            2
            +
            lineHeight / 2;


        lines.forEach(line=>{

            ctx.fillText(

                line,

                this.canvas.width / 2,

                y

            );

            y += lineHeight;

        });


        ctx.restore();


    }


}
