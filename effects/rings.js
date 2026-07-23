/*
==================================================
DANCE TRACKER 5000
AMIGA RINGS EFFECT
==================================================
*/

export class Rings {

    constructor(settings) {
        this.settings = settings;
        this.time = 0;

        this.canvas = document.getElementById("overlay-layer");
        this.ctx = this.canvas.getContext("2d");
    }


    update() {
        this.time += 0.03;
    }


    draw() {

        const rings = this.settings.amiga.rings;

        if( !rings.enabled || !this.settings.layers.effects )
            return;


        const ctx = this.ctx;

        ctx.clearRect(
            0,
            0,
            this.canvas.width,
            this.canvas.height
        );


        const cx = this.canvas.width / 2;
        const cy = this.canvas.height / 2;


        const colours = [
            "#ff00ff",
            "#00ffff",
            "#ffff00",
            "#ff6600"
        ];


        for (let i = 0; i < rings.count; i++) {

            const radius =
                rings.size +
                i * 20 +
                Math.sin(
                    this.time + i
                ) * 15;


            ctx.beginPath();

            ctx.arc(
                cx,
                cy,
                radius,
                0,
                Math.PI * 2
            );


            ctx.strokeStyle =
                colours[
                    i % colours.length
                ];


            ctx.lineWidth =
                rings.width;


            ctx.globalAlpha = 0.75;

            ctx.stroke();

        }


        ctx.globalAlpha = 1;

    }


    setEnabled(value) {
        this.settings.amiga.rings.enabled = value;
    }

}
