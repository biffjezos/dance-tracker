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



        this.path = [];



        this.menus = {


            project:[

                "CAMERA START",

                "CAMERA STOP",

                "CAMERA CAPTURE BACKGROUND",

                "RECORD START",

                "RECORD STOP"

            ],



            video:{

                "VIDEO ON/OFF":null,



                BACKGROUND:[

                    "BG BLACK",

                    "BG GREEN",

                    "BG BLUE"

                ]

            },



            body:[

                "BODY ON/OFF",

                "THRESHOLD +",

                "THRESHOLD -",

                "MAGENTA",

                "GREEN",

                "BLUE"

            ],



            amiga:{

                RINGS:[

                    "RINGS ON",

                    "RINGS OFF",

                    "RING COUNT +",

                    "RING COUNT -",

                    "RING SIZE +",

                    "RING SIZE -",

                    "CONSTELLATION ON",

                    "CONSTELLATION OFF",

                    "CONSTELLATION DISTANCE +",

                    "CONSTELLATION DISTANCE -"

                ],



                GHOST:[

                    "GHOST +",

                    "GHOST -",

                    "GHOST DELAY +",

                    "GHOST DELAY -"

                ]

            }

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


        this.path = [menuName];


        this.render();


    }




    enter(category){


        this.path.push(
            category
        );


        this.render();


    }




    up(){


        if(this.path.length > 1){

            this.path.pop();

            this.render();

        }


    }




    node(){


        let node = this.menus;


        this.path.forEach(key=>{

            node = node[key];

        });


        return node;


    }




    render(){


        this.subMenu.innerHTML="";


        let node =
            this.node();



        if(this.path.length > 1){


            let up =
            document.createElement(
                "button"
            );


            up.innerText="UP";

            up.className="up-button";


            up.onclick=()=>{

                this.up();

            };


            this.subMenu.appendChild(
                up
            );


        }



        if(Array.isArray(node)){


            node.forEach(item=>{


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
        else {


            Object.keys(node)
            .forEach(key=>{


                let button =
                document.createElement(
                    "button"
                );


                button.innerText=key;


                if(Array.isArray(node[key])){

                    button.onclick=()=>{

                        this.enter(
                            key
                        );

                    };

                }
                else {

                    button.onclick=()=>{

                        this.select(
                            key
                        );

                    };

                }


                this.subMenu.appendChild(
                    button
                );


            });


        }


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



        if(item==="CONSTELLATION ON")
            window.dispatchEvent(
                new Event("constellationOn")
            );



        if(item==="CONSTELLATION OFF")
            window.dispatchEvent(
                new Event("constellationOff")
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



        if(item==="BG BLACK")
            window.dispatchEvent(
                new CustomEvent(
                    "videoBackgroundColour",
                    {
                        detail:{
                            r:0,
                            g:0,
                            b:0
                        }
                    }
                )
            );



        if(item==="BG GREEN")
            window.dispatchEvent(
                new CustomEvent(
                    "videoBackgroundColour",
                    {
                        detail:{
                            r:0,
                            g:20,
                            b:0
                        }
                    }
                )
            );



        if(item==="BG BLUE")
            window.dispatchEvent(
                new CustomEvent(
                    "videoBackgroundColour",
                    {
                        detail:{
                            r:0,
                            g:10,
                            b:30
                        }
                    }
                )
            );



        if(item==="CONSTELLATION DISTANCE +")
            window.dispatchEvent(
                new Event("constellationDistanceUp")
            );



        if(item==="CONSTELLATION DISTANCE -")
            window.dispatchEvent(
                new Event("constellationDistanceDown")
            );



        if(item==="GHOST DELAY +")
            window.dispatchEvent(
                new Event("ghostDelayUp")
            );



        if(item==="GHOST DELAY -")
            window.dispatchEvent(
                new Event("ghostDelayDown")
            );


    }

}