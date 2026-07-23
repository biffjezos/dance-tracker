/*
==================================================
DANCE TRACKER 5000
1990s STYLE BODY SEGMENTATION
==================================================
*/

export class Segmentation {

    constructor(background, settings, palette) {
        this.background = background;
        this.settings = settings;
        this.palette = palette;
        this.canvas = document.createElement("canvas");
        this.canvas.width = 320;
        this.canvas.height = 240;

        this.ctx = this.canvas.getContext("2d");

        this.output = document.getElementById("body-layer");
        this.outputCtx = this.output.getContext("2d");

    }

    getColour(){

        let colour =
            this.palette.get().body;


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
    process(video) {
        if(!this.settings.layers.body)
            return;
        if (!this.background.hasBackground)
            return;

        this.ctx.drawImage(video, 0, 0, 320, 240);

        const current = this.ctx.getImageData(
            0,
            0,
            320,
            240
        );

        const bg = this.background.canvas
            .getContext("2d")
            .getImageData(
                0,
                0,
                320,
                240
            );


        const pixels = current.data;
        const bgPixels = bg.data;

        const result = this.outputCtx.createImageData(
            320,
            240
        );


        const threshold = this.settings.body.threshold;


        for (let i = 0; i < pixels.length; i += 4) {

            const difference =
                Math.abs(pixels[i] - bgPixels[i]) +
                Math.abs(pixels[i + 1] - bgPixels[i + 1]) +
                Math.abs(pixels[i + 2] - bgPixels[i + 2]);


            if (difference > threshold) {

                let colour = this.getColour();

                result.data[i] =
                    colour.r;
                result.data[i+1] =
                    colour.g;
                result.data[i+2] =
                    colour.b;
                result.data[i + 3] = 255;

            }

        }


        this.outputCtx.putImageData(
            result,
            0,
            0
        );

    }


    setColour(r, g, b) {
        this.colour.r = r;
        this.colour.g = g;
        this.colour.b = b;
        console.log( "Body colour:", r, g, b);
    }
}
