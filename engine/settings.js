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

            height:240


        };





        this.body = {
            mode:"1990",
            colourMode:"magenta",
            blendMode:"normal",
            threshold:40
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

                amount:0.2


            }



        };







        this.amiga = {


            rings:{


                enabled:false,


                count:2,


                speed:2,


                size:80,


                width:6,


                blend:"screen"


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


        let parts = path.split(".");


        let obj=this;



        while(parts.length>1){


            obj=obj[parts.shift()];


        }



        obj[parts[0]]=value;


    }



}