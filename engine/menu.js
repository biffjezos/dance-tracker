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

    {label:"YELLOW", r:255, g:255, b:0},

    {label:"DARK GREEN", r:0, g:20, b:0},

    {label:"DARK BLUE", r:0, g:10, b:30}

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


        this.ringId = 1;



        this.menus = {


            input:{

                CAMERA:{

                    "CAMERA ON/OFF":null,

                    "CAPTURE BACKGROUND":null

                }

            },



            video:{

                "VIDEO ON/OFF":null,



                BACKGROUND:{

                    COLOUR:colourMenu(
                        "videoBackgroundColour"
                    )

                }

            },



            amiga:{

                BODY:{

                    "BODY ON/OFF":null,

                    "THRESHOLD +":null,

                    "THRESHOLD -":null,



                    COLOUR:colourMenu(
                        "bodyColour"
                    ),



                    GHOST:[

                        "GHOST +",

                        "GHOST -",

                        "GHOST DELAY +",

                        "GHOST DELAY -"

                    ]

                },



                RINGS:{

                    "RINGS ON/OFF":null,

                    "RING COUNT +":null,

                    "RING COUNT -":null,

                    "RING SIZE +":null,

                    "RING SIZE -":null,

                    "RING THICKNESS +":null,

                    "RING THICKNESS -":null,



                    CONSTELLATION:[

                        "ON/OFF",

                        "DISTANCE +",

                        "DISTANCE -"

                    ],



                    COLOUR:{

                        type:"ringColourPicker"

                    }

                },



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

            },



            output:{

                RECORDER:{

                    "RECORD ON/OFF":null,

                    "OUTPUT SIZE +":null,

                    "OUTPUT SIZE -":null

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
            "input"
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



        if(
            node &&
            node.type==="ringColourPicker"
        ){

            this.renderRingColourPicker();

            return;

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


                    this.styleSwatch(
                        button,
                        item.r,
                        item.g,
                        item.b
                    );


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




    styleSwatch(button, r, g, b){


        button.style.backgroundColor=
            "rgb("
            +
            r
            +
            ","
            +
            g
            +
            ","
            +
            b
            +
            ")";


        const brightness=
            (
                r * 299
                +
                g * 587
                +
                b * 114
            )
            /
            1000;


        button.style.color=
            brightness > 150
            ?
            "#000"
            :
            "#fff";


    }




    renderRingColourPicker(){


        let minus =
        document.createElement(
            "button"
        );


        minus.innerText=
            "RING ID -";


        minus.onclick=()=>{

            if(this.ringId > 1)
                this.ringId--;

            this.render();

        };


        this.subMenu.appendChild(
            minus
        );



        let display =
        document.createElement(
            "span"
        );


        display.innerText=
            "RING " +
            this.ringId;


        display.className=
            "ring-id-display";


        this.subMenu.appendChild(
            display
        );



        let plus =
        document.createElement(
            "button"
        );


        plus.innerText=
            "RING ID +";


        plus.onclick=()=>{

            if(this.ringId < 8)
                this.ringId++;

            this.render();

        };


        this.subMenu.appendChild(
            plus
        );



        SWATCHES.forEach(swatch=>{


            let button =
            document.createElement(
                "button"
            );


            button.innerText=
                swatch.label;


            this.styleSwatch(
                button,
                swatch.r,
                swatch.g,
                swatch.b
            );


            button.onclick=()=>{

                console.log(
                    "MENU SELECTED:",
                    "RING",
                    this.ringId,
                    swatch.label
                );

                window.dispatchEvent(
                    new CustomEvent(
                        "ringColour",
                        {
                            detail:{
                                ringId:this.ringId,
                                r:swatch.r,
                                g:swatch.g,
                                b:swatch.b
                            }
                        }
                    )
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



        if(item==="CAMERA ON/OFF")
            window.dispatchEvent(
                new Event("toggleCamera")
            );



        if(item==="CAPTURE BACKGROUND")
            window.dispatchEvent(
                new Event("captureBackground")
            );



        if(item==="RECORD ON/OFF")
            window.dispatchEvent(
                new Event("toggleRecord")
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



        if(item==="RING THICKNESS +")
            window.dispatchEvent(
                new Event("ringThicknessUp")
            );



        if(item==="RING THICKNESS -")
            window.dispatchEvent(
                new Event("ringThicknessDown")
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



        if(item==="OUTPUT SIZE +")
            window.dispatchEvent(
                new Event("outputSizeUp")
            );



        if(item==="OUTPUT SIZE -")
            window.dispatchEvent(
                new Event("outputSizeDown")
            );



    }

}