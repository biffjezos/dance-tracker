/*
==================================================
DANCE TRACKER 5000
AMIGA TWO ROW MENU SYSTEM
==================================================
*/


const SWATCHES = [

    {label:"BLACK", r:0, g:0, b:0},

    {label:"WHITE", r:255, g:255, b:255},

    {label:"RED", r:255, g:0, b:0},

    {label:"GREEN", r:0, g:255, b:0},

    {label:"BLUE", r:0, g:150, b:255},

    {label:"MAGENTA", r:255, g:0, b:255},

    {label:"CYAN", r:0, g:255, b:255},

    {label:"YELLOW", r:255, g:255, b:0}

];



function colourMenu(event){


    return SWATCHES.map(swatch=>({

        label:swatch.label,

        event:event,

        r:swatch.r,

        g:swatch.g,

        b:swatch.b

    }));


}



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



                BACKGROUND:{

                    COLOUR:colourMenu(
                        "videoBackgroundColour"
                    )

                }

            },



            body:{

                "BODY ON/OFF":null,

                "THRESHOLD +":null,

                "THRESHOLD -":null,



                COLOUR:colourMenu(
                    "bodyColour"
                )

            },



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



                    COLOUR:colourMenu(
                        "setTextColour"
                    )

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


                if(typeof item === "object"){


                    button.innerText=
                        item.label;


                    button.style.backgroundColor=
                        "rgb("
                        +
                        item.r
                        +
                        ","
                        +
                        item.g
                        +
                        ","
                        +
                        item.b
                        +
                        ")";


                    const brightness=
                        (
                            item.r * 299
                            +
                            item.g * 587
                            +
                            item.b * 114
                        )
                        /
                        1000;


                    button.style.color=
                        brightness > 150
                        ?
                        "#000"
                        :
                        "#fff";


                    button.onclick=()=>{

                        console.log(
                            "MENU SELECTED:",
                            item.label
                        );

                        window.dispatchEvent(
                            new CustomEvent(
                                item.event,
                                {
                                    detail:{
                                        r:item.r,
                                        g:item.g,
                                        b:item.b
                                    }
                                }
                            )
                        );

                    };


                }
                else {


                    button.innerText=item;


                    button.onclick=()=>{

                        this.select(
                            item
                        );

                    };


                }


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



    }

}