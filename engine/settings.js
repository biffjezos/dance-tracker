/*
==================================================
DANCE TRACKER 5000
SETTINGS ENGINE
==================================================
*/

export class Settings {

    constructor(){

        this.video = {
            mirror:false,
            width:320,
            height:240,
            enabled:true,
            backgroundColour:"rgb(0,0,0)"
        };


        this.layers = {
            body:true,
            effects:true
        };


        this.body = {
            mode:"difference",
            colourMode:"magenta",
            blendMode:"normal",
            threshold:100,
            keyColour:{r:0, g:255, b:0},
            fill:"solid"
        };


        this.effects = {

            pixelate:{
                enabled:false,
                size:4
            },


            scanlines:{
                enabled:false,
                height:4
            },


            rgbShift:{
                enabled:false,
                amount:5
            },


            ghost:{
                enabled:false,
                amount:0.3
            }

        };



        this.amiga = {

            rings:{
                enabled:false,
                count:2,
                ringsPerGroup:8,
                spacing:14,
                speed:2,
                size:20,
                width:6,
                blend:"screen",
                colours:[
                    "rgb(255,0,255)",
                    "rgb(0,255,80)",
                    "rgb(255,0,255)",
                    "rgb(0,255,80)",
                    "rgb(255,0,255)",
                    "rgb(0,255,80)",
                    "rgb(255,0,255)",
                    "rgb(0,255,80)"
                ],
                constellation:{
                    enabled:false,
                    distance:70
                }
            },

            ghost:{
                enabled:false,
                count:0,
                alpha:0.45,
                delay:50
            },

            text:{
                content:"",
                colour:"rgb(255,255,255)",
                size:24
            },
            plasma:{
                enabled:false,
                speed:1
            },


            copper:{
                enabled:false
            },


            stars:{
                enabled:false,
                count:100
            },


            vectorBalls:{
                enabled:false,
                count:40
            }

        };



        this.audio = {

            enabled:false,
            bpm:128

        };



        this.recording = {

            enabled:false

        };



        this.debug = {

            fps:true

        };

    }



    get(path){

        return path
            .split(".")
            .reduce(
                (obj,key)=>obj[key],
                this
            );

    }



    set(path,value){

        let parts =
            path.split(".");


        let obj=this;


        while(parts.length > 1){

            obj =
                obj[parts.shift()];

        }


        obj[parts[0]] = value;

    }

}
