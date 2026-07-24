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


        this.textValue = "";



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

                    "BG WHITE",

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

                RINGS:{

                    "RINGS ON/OFF":null,

                    "RING COUNT +":null,

                    "RING COUNT -":null,

                    "RING SIZE +":null,

                    "RING SIZE -":null,



                    CONSTELLATION:[

                        "ON/OFF",

                        "DISTANCE +",

                        "DISTANCE -"

                    ]

                },



                GHOST:[

                    "GHOST +",

                    "GHOST -",

                    "GHOST DELAY +",

                    "GHOST DELAY -"

                ],



                TEXT:{

                    "ENTER TEXT":{

                        type:"input",

                        placeholder:"ENTER YOUR TEXT HERE",

                        event:"setText"

                    },

                    "SIZE +":null,

                    "SIZE -":null,



                    "FONT COLOUR":[

                        "FONT WHITE",

                        "FONT BLACK",

                        "FONT MAGENTA",

                        "FONT CYAN",

                        "FONT YELLOW"

                    ]

                }

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


                let value =
                    node[key];


                if(
                    value &&
                    value.type==="input"
                ){


                    let input =
                    document.createElement(
                        "input"
                    );


                    input.type="text";

                    input.placeholder=
                        value.placeholder ||
                        "";

                    input.maxLength=200;

                    input.className=
                        "menu-input";

                    input.value=
                        this.textValue;


                    input.addEventListener(
                        "input",
                        ()=>{

                            this.textValue=
                                input.value;

                            window.dispatchEvent(
                                new CustomEvent(
                                    value.event,
                                    {
                                        detail:{
                                            value:
                                            input.value
                                        }
                                    }
                                )
                            );

                        }
                    );


                    this.subMenu.appendChild(
                        input
                    );


                }
                else if(
                    value &&
                    typeof value === "object"
                ){


                    let button =
                    document.createElement(
                        "button"
                    );


                    button.innerText=key;


                    button.onclick=()=>{

                        this.enter(
                            key
                        );

                    };


                    this.subMenu.appendChild(
                        button
                    );


                }
                else {


                    let button =
                    document.createElement(
                        "button"
                    );


                    button.innerText=key;


                    button.onclick=()=>{

                        this.select(
                            key
                        );

                    };


                    this.subMenu.appendChild(
                        button
                    );


                }


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



        if(item==="RINGS ON/OFF")
            window.dispatchEvent(
                new Event("toggleRings")
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



        if(item==="ON/OFF")
            window.dispatchEvent(
                new Event("toggleConstellation")
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



        if(item==="BG WHITE")
            window.dispatchEvent(
                new CustomEvent(
                    "videoBackgroundColour",
                    {
                        detail:{
                            r:255,
                            g:255,
                            b:255
                        }
                    }
                )
            );



        if(item==="DISTANCE +")
            window.dispatchEvent(
                new Event("constellationDistanceUp")
            );



        if(item==="DISTANCE -")
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



        if(item==="SIZE +")
            window.dispatchEvent(
                new Event("textSizeUp")
            );



        if(item==="SIZE -")
            window.dispatchEvent(
                new Event("textSizeDown")
            );



        if(item==="FONT WHITE")
            window.dispatchEvent(
                new CustomEvent(
                    "setTextColour",
                    {
                        detail:{
                            r:255,
                            g:255,
                            b:255
                        }
                    }
                )
            );



        if(item==="FONT BLACK")
            window.dispatchEvent(
                new CustomEvent(
                    "setTextColour",
                    {
                        detail:{
                            r:0,
                            g:0,
                            b:0
                        }
                    }
                )
            );



        if(item==="FONT MAGENTA")
            window.dispatchEvent(
                new CustomEvent(
                    "setTextColour",
                    {
                        detail:{
                            r:255,
                            g:0,
                            b:255
                        }
                    }
                )
            );



        if(item==="FONT CYAN")
            window.dispatchEvent(
                new CustomEvent(
                    "setTextColour",
                    {
                        detail:{
                            r:0,
                            g:255,
                            b:255
                        }
                    }
                )
            );



        if(item==="FONT YELLOW")
            window.dispatchEvent(
                new CustomEvent(
                    "setTextColour",
                    {
                        detail:{
                            r:255,
                            g:255,
                            b:0
                        }
                    }
                )
            );


    }

}