/*
==================================================
DANCE TRACKER 5000
1990s BACKGROUND CAPTURE
==================================================
*/


export class BackgroundCapture {
    constructor(){
        this.canvas = document.createElement( "canvas" );
        this.canvas.width=320;
        this.canvas.height=240;
        this.ctx = this.canvas.getContext( "2d" );
        this.hasBackground=false;
    }

    capture(video) {
        this.ctx.drawImage( video, 0, 0, 320, 240 );
        this.hasBackground=true;
        console.log( "Background captured"  );
    }
    getFrame(){
        if(!this.hasBackground)
            return null;

        return this.canvas;
    }
}