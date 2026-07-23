/*
==================================================
DANCE TRACKER 5000
PALETTE ENGINE
==================================================
*/

export class Palette {

    constructor(){

        this.palettes = {

            classicAmiga:{

                background:"#000000",

                body:"#ff00ff",

                ghost:"#ff00ff",

                rings:[
                    "#ff00ff",
                    "#00ff66"
                ]

            },


            cyberGreen:{

                background:"#001000",

                body:"#00ff66",

                ghost:"#00ff66",

                rings:[
                    "#00ff66",
                    "#ffffff"
                ]

            },


            iceBlue:{

                background:"#001020",

                body:"#00ccff",

                ghost:"#00ccff",

                rings:[
                    "#00ccff",
                    "#ffffff"
                ]

            }

        };


        this.current =
            "classicAmiga";


    }



    get(){

        return this.palettes[
            this.current
        ];

    }



    next(){

        let keys =
            Object.keys(
                this.palettes
            );


        let index =
            keys.indexOf(
                this.current
            );


        index++;


        if(index >= keys.length)
            index = 0;


        this.current =
            keys[index];

    }



    set(name){

        if(this.palettes[name]){

            this.current=name;

        }

    }

}