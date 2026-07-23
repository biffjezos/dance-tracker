/*
==================================================
DANCE TRACKER 5000
AMIGA TWO ROW MENU SYSTEM
==================================================
*/

export class MenuManager {

    constructor(){

        this.subMenu =
        document.getElementById("sub-menu");


        this.menus = {

            project:[
                "START CAMERA",
                "STOP CAMERA",
                "CAPTURE BACKGROUND",
                "RECORD",
                "STOP RECORD",
                "DOWNLOAD"
            ],


            video:[
                "VIDEO ON/OFF",
                "MIRROR",
                "BRIGHTNESS",
                "CONTRAST"
            ],


            body:[
                "OFF",
                "1990 VIDEO LAB",
                "MODERN AI",
                "HYBRID",
                "THRESHOLD +",
                "THRESHOLD -",
                "MAGENTA",
                "GREEN",
                "BLUE"
            ],


            amiga:[
                "RINGS ON",
                "RINGS OFF",
                "RING COUNT +",
                "RING COUNT -",
                "RING SIZE +",
                "RING SIZE -",
                "GHOST +",
                "GHOST -",
                "PLASMA",
                "STARFIELD",
                "VECTOR BALLS",
                "TUNNEL",
                "ROTOZOOM"
            ],


            fx:[
                "PIXELATE",
                "SCANLINES",
                "RGB SHIFT",
                "CRT"
            ],


            audio:[
                "MIC INPUT",
                "BEAT DETECTION",
                "BPM"
            ],


            presets:[
                "1992 RAVE",
                "PLASMA DREAM",
                "CYBERPUNK",
                "DEMO PARTY"
            ],


            help:[
                "ABOUT",
                "CONTROLS"
            ]

        };

    }



    init(){

        const buttons =
        document.querySelectorAll(
            ".main-menu button"
        );


        buttons.forEach(button=>{

            button.addEventListener(
                "click",
                ()=>{

                    this.show(
                        button.dataset.menu
                    );

                }
            );

        });


        this.show("project");

    }




    show(menuName){

        this.subMenu.innerHTML="";


        this.menus[menuName]
        .forEach(item=>{

            let button =
            document.createElement(
                "button"
            );


            button.innerText=item;


            button.addEventListener(
                "click",
                ()=>{

                    this.select(item);

                }
            );


            this.subMenu.appendChild(
                button
            );

        });

    }





    select(item){

        console.log(
            "MENU SELECTED:",
            item
        );


        if(item === "START CAMERA"){
            window.dispatchEvent(
                new Event("startCamera")
            );
        }


        if(item === "STOP CAMERA"){
            window.dispatchEvent(
                new Event("stopCamera")
            );
        }


        if(item === "VIDEO ON/OFF"){
            window.dispatchEvent(
                new Event("toggleVideo")
            );
        }


        if(item === "CAPTURE BACKGROUND"){
            window.dispatchEvent(
                new Event("captureBackground")
            );
        }



        if(item === "THRESHOLD +"){
            window.dispatchEvent(
                new Event("thresholdUp")
            );
        }


        if(item === "THRESHOLD -"){
            window.dispatchEvent(
                new Event("thresholdDown")
            );
        }



        if(item === "MAGENTA"){
            window.dispatchEvent(
                new CustomEvent(
                    "bodyColour",
                    {
                        detail:{
                            r:255,
                            g:0,
                            b:255
                        }
                    }
                )
            );
        }



        if(item === "GREEN"){
            window.dispatchEvent(
                new CustomEvent(
                    "bodyColour",
                    {
                        detail:{
                            r:0,
                            g:255,
                            b:0
                        }
                    }
                )
            );
        }



        if(item === "BLUE"){
            window.dispatchEvent(
                new CustomEvent(
                    "bodyColour",
                    {
                        detail:{
                            r:0,
                            g:120,
                            b:255
                        }
                    }
                )
            );
        }



        if(item === "RECORD"){
            window.dispatchEvent(
                new Event("startRecord")
            );
        }


        if(item === "STOP RECORD"){
            window.dispatchEvent(
                new Event("stopRecord")
            );
        }



        if(item === "RINGS ON"){
            window.dispatchEvent(
                new Event("ringsOn")
            );
        }


        if(item === "RINGS OFF"){
            window.dispatchEvent(
                new Event("ringsOff")
            );
        }


        if(item === "RING COUNT +"){
            window.dispatchEvent(
                new Event("ringCountUp")
            );
        }


        if(item === "RING COUNT -"){
            window.dispatchEvent(
                new Event("ringCountDown")
            );
        }


        if(item === "RING SIZE +"){
            window.dispatchEvent(
                new Event("ringSizeUp")
            );
        }


        if(item === "RING SIZE -"){
            window.dispatchEvent(
                new Event("ringSizeDown")
            );
        }
        if(item === "GHOST +"){
            window.dispatchEvent(
                new Event("ghostUp")
            );
        }
        
        
        if(item === "GHOST -"){
            window.dispatchEvent(
                new Event("ghostDown")
            );
        }
    }

}
