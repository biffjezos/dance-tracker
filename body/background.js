/*
==================================================
DANCE TRACKER 5000
1990s BACKGROUND CAPTURE
==================================================
*/


import { containFit } from "../engine/fit.js";


export class BackgroundCapture {
    constructor(settings){
        this.settings = settings;
        this.canvas = document.createElement( "canvas" );
        this.canvas.width = settings.video.width;
        this.canvas.height = settings.video.height;
        this.ctx = this.canvas.getContext( "2d" );
        this.hasBackground=false;
    }

    capture(video) {
        this.ctx.clearRect( 0, 0, this.canvas.width, this.canvas.height );

        const rect =
            containFit(
                video.videoWidth,
                video.videoHeight,
                this.canvas.width,
                this.canvas.height
            );

        this.ctx.drawImage( video, rect.x, rect.y, rect.width, rect.height );
        this.hasBackground=true;
        console.log( "Background captured"  );
    }

    resize(){
        this.canvas.width = this.settings.video.width;
        this.canvas.height = this.settings.video.height;
        this.hasBackground = false;
    }

    getFrame(){
        if(!this.hasBackground)
            return null;

        return this.canvas;
    }
}