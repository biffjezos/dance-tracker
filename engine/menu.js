/*
==================================================
DANCE TRACKER 5000
AMIGA TWO ROW MENU SYSTEM
==================================================
*/

export class MenuManager {

    constructor(){


        this.subMenu =
        document.getElementById(
            "sub-menu"
        );



        this.menus = {


            project:[

                "CAMERA START",

                "CAMERA STOP",

                "CAMERA CAPTURE BACKGROUND",

                "RECORD START",

                "RECORD STOP"

            ],



            video:[

                "VIDEO ON/OFF"

            ],



            body:[

                "BODY ON/OFF",

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

                "GHOST -"

            ],



            presets:[

                "CLASSIC AMIGA",

                "CYBER GREEN",

                "ICE BLUE"

            ]

        };

    }



    init(){


        document
        .querySelectorAll(
            ".main-menu button"
        )
        .forEach(button=>{


            button.addEventListener(
                "click",
                ()=>{

                    this.show(
                        button.dataset.menu
                    );

                }
            );


        });



        this.show(
            "project"
        );

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


            button.onclick=()=>{

                this.select(
                    item
                );

            };


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



        if(item==="CAMERA START")
            window.dispatchEvent(
                new Event("startCamera")
            );



        if(item==="CAMERA STOP")
            window.dispatchEvent(
                new Event("stopCamera")
            );



        if(item==="CAMERA CAPTURE BACKGROUND")
            window.dispatchEvent(
                new Event("captureBackground")
            );



        if(item==="VIDEO ON/OFF")
            window.dispatchEvent(
                new Event("toggleVideo")
            );



        if(item==="BODY ON/OFF")
            window.dispatchEvent(
                new Event("toggleBody")
            );



        if(item==="THRESHOLD +")
            window.dispatchEvent(
                new Event("thresholdUp")
            );



        if(item==="THRESHOLD -")
            window.dispatchEvent(
                new Event("thresholdDown")
            );



        if(item==="MAGENTA")
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



        if(item==="GREEN")
            window.dispatchEvent(
                new CustomEvent(
                    "bodyColour",
                    {
                        detail:{
                            r:0,
                            g:255,
                            b:80
                        }
                    }
                )
            );



        if(item==="BLUE")
            window.dispatchEvent(
                new CustomEvent(
                    "bodyColour",
                    {
                        detail:{
                            r:0,
                            g:150,
                            b:255
                        }
                    }
                )
            );



        if(item==="RINGS ON")
            window.dispatchEvent(
                new Event("ringsOn")
            );



        if(item==="RINGS OFF")
            window.dispatchEvent(
                new Event("ringsOff")
            );



        if(item==="RING COUNT +")
            window.dispatchEvent(
                new Event("ringCountUp")
            );



        if(item==="RING COUNT -")
            window.dispatchEvent(
                new Event("ringCountDown")
            );



        if(item==="RING SIZE +")
            window.dispatchEvent(
                new Event("ringSizeUp")
            );



        if(item==="RING SIZE -")
            window.dispatchEvent(
                new Event("ringSizeDown")
            );



        if(item==="GHOST +")
            window.dispatchEvent(
                new Event("ghostUp")
            );



        if(item==="GHOST -")
            window.dispatchEvent(
                new Event("ghostDown")
            );



        if(item==="RECORD START")
            window.dispatchEvent(
                new Event("startRecord")
            );



        if(item==="RECORD STOP")
            window.dispatchEvent(
                new Event("stopRecord")
            );



        if(item==="CLASSIC AMIGA")
            window.dispatchEvent(
                new CustomEvent(
                    "setPalette",
                    {
                        detail:{
                            name:"classicAmiga"
                        }
                    }
                )
            );



        if(item==="CYBER GREEN")
            window.dispatchEvent(
                new CustomEvent(
                    "setPalette",
                    {
                        detail:{
                            name:"cyberGreen"
                        }
                    }
                )
            );



        if(item==="ICE BLUE")
            window.dispatchEvent(
                new CustomEvent(
                    "setPalette",
                    {
                        detail:{
                            name:"iceBlue"
                        }
                    }
                )
            );


    }

}