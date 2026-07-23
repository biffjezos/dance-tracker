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

                "RECORD",

                "STOP",

                "DOWNLOAD"

            ],




            video:[

                "MIRROR",

                "BRIGHTNESS",

                "CONTRAST"

            ],




            body:[

                "NORMAL",

                "1990 VIDEO LAB",

                "MODERN AI",

                "HYBRID",

                "COLOUR",

                "OUTLINE"

            ],




            amiga:[

                "RINGS",

                "PLASMA",

                "COPPER",

                "STARFIELD",

                "VECTOR BALLS",

                "TUNNEL",

                "ROTOZOOM"

            ],




            fx:[

                "PIXELATE",

                "SCANLINES",

                "RGB SHIFT",

                "GHOST",

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



            let button=

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



    }




}