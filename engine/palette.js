/*
==================================================
DANCE TRACKER 5000
PALETTE ENGINE
==================================================
*/


export class Palette {


    constructor(){


        this.palettes = {


            classicAmiga: {

                background: "rgb(0,0,0)",

                body: "rgb(255,0,255)",

                ghost: "rgb(255,0,255)",

                rings: [
                    "rgb(255,0,255)",
                    "rgb(0,255,80)"
                ]

            },


            cyberGreen: {

                background: "rgb(0,20,0)",

                body: "rgb(0,255,80)",

                ghost: "rgb(0,255,80)",

                rings: [
                    "rgb(0,255,80)",
                    "rgb(255,255,255)"
                ]

            },


            iceBlue: {

                background: "rgb(0,10,30)",

                body: "rgb(0,200,255)",

                ghost: "rgb(0,200,255)",

                rings: [
                    "rgb(0,200,255)",
                    "rgb(255,255,255)"
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
        if(index >= keys.length){
            index = 0;
        }

        this.current =
            keys[index];

        console.log(
            "Palette:",
            this.current
        );


    }

    set(name){
        if(
            this.palettes[name]
        ){

            this.current =
                name;

            console.log(
                "Palette:",
                this.current
            );
        }
    }
}