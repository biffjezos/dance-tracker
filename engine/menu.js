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


        this.keyColour = {r:0, g:255, b:0};


        window.addEventListener(
            "bodyKeyColour",
            e=>{

                this.keyColour = {
                    r:e.detail.r,
                    g:e.detail.g,
                    b:e.detail.b
                };


                let node =
                    this.node();

                if(
                    node &&
                    node.type==="keyColourPicker"
                ){

                    this.render();

                }

            }
        );


        this.layerSelection = {index:0, name:"VIDEO 1"};


        window.addEventListener(
            "layerSelectionChanged",
            e=>{

                this.layerSelection = {
                    index:e.detail.index,
                    name:e.detail.name
                };

                this.keyColour = {
                    r:e.detail.keyColour.r,
                    g:e.detail.keyColour.g,
                    b:e.detail.keyColour.b
                };


                if(
                    this.path[0] === "key" ||
                    this.path[0] === "video"
                ){

                    this.render();

                }

            }
        );


        this.maskState = {

            video:{source:"none", sourceLabel:"NONE", channel:"alpha"},

            body:{source:"none", sourceLabel:"NONE", channel:"alpha"},

            rings:{source:"none", sourceLabel:"NONE", channel:"alpha"},

            text:{source:"none", sourceLabel:"NONE", channel:"alpha"}

        };


        window.addEventListener(
            "maskSettingsChanged",
            e=>{

                this.maskState[e.detail.layer] = {
                    source:e.detail.source,
                    sourceLabel:e.detail.sourceLabel,
                    channel:e.detail.channel
                };


                let node =
                    this.node();

                if(
                    node &&
                    node.type==="maskPicker" &&
                    node.layer===e.detail.layer
                ){

                    this.render();

                }

            }
        );



        this.menus = {


            input:{

                CAMERA:{

                    "CAMERA ON/OFF":null

                },

                "LOAD VIDEO":{

                    type:"fileInput",

                    accept:"video/*",

                    event:"loadVideoFile"

                },

                "LOAD AUDIO":{

                    type:"fileInput",

                    accept:"audio/*",

                    event:"loadAudioFile"

                },

                "ADD VIDEO LAYER":{

                    type:"fileInput",

                    accept:"video/*",

                    event:"addVideoLayer"

                },

                TRANSPORT:{

                    "PLAY/STOP":null,

                    TIME:{

                        type:"display",

                        id:"transport-display"

                    },

                    "MINUTE +":null,

                    "MINUTE -":null,

                    "SECOND +":null,

                    "SECOND -":null,

                    "FRAME +":null,

                    "FRAME -":null,



                    "AUDIO SYNC":{

                        OFFSET:{

                            type:"display",

                            id:"audio-sync-display"

                        },

                        "SYNC MINUTE +":null,

                        "SYNC MINUTE -":null,

                        "SYNC SECOND +":null,

                        "SYNC SECOND -":null,

                        "SYNC FRAME +":null,

                        "SYNC FRAME -":null

                    }

                }

            },



            video:{

                LAYER:{

                    type:"layerStepper"

                },

                "VIDEO ON/OFF":null,

                "VIDEO VISIBLE ON/OFF":null,



                BACKGROUND:{

                    COLOUR:colourMenu(
                        "videoBackgroundColour"
                    )

                },


                "VIDEO MASKED BY":{

                    type:"maskPicker",

                    layer:"video"

                },



                "BODY ON/OFF":null,

                "BODY VISIBLE ON/OFF":null,


                "BODY MASKED BY":{

                    type:"maskPicker",

                    layer:"body"

                }

            },



            key:{

                LAYER:{

                    type:"layerStepper"

                },

                "CAPTURE BACKGROUND":null,

                "THRESHOLD +":null,

                "THRESHOLD -":null,

                "DIFFERENCE/CHROMA":null,

                "SOLID/VIDEO":null,

                COLOUR:colourMenu(
                    "bodyColour"
                ),

                "KEY COLOUR":{

                    type:"keyColourPicker"

                }

            },



            generate:{

                GHOST:[

                    "GHOST +",

                    "GHOST -",

                    "GHOST DELAY +",

                    "GHOST DELAY -"

                ],



                RINGS:{

                    "RINGS ON/OFF":null,

                    "RINGS VISIBLE ON/OFF":null,

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

                    },


                    "MASKED BY":{

                        type:"maskPicker",

                        layer:"rings"

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
                    ),


                    "MASKED BY":{

                        type:"maskPicker",

                        layer:"text"

                    }

                }

            },



            transform:{

                "COMING SOON":null

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



        if(
            node &&
            node.type==="keyColourPicker"
        ){

            this.renderKeyColourPicker();

            return;

        }



        if(
            node &&
            node.type==="maskPicker"
        ){

            this.renderMaskPicker(
                node.layer
            );

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
                    value.type==="fileInput"
                ){


                    let button =
                    document.createElement(
                        "button"
                    );


                    button.innerText=key;


                    button.onclick=()=>{

                        let fileInput =
                        document.createElement(
                            "input"
                        );

                        fileInput.type=
                            "file";

                        fileInput.accept=
                            value.accept ||
                            "";

                        fileInput.addEventListener(
                            "change",
                            ()=>{

                                if(fileInput.files.length === 0)
                                    return;

                                console.log(
                                    "MENU SELECTED:",
                                    key
                                );

                                window.dispatchEvent(
                                    new CustomEvent(
                                        value.event,
                                        {
                                            detail:{
                                                file:
                                                fileInput.files[0]
                                            }
                                        }
                                    )
                                );

                            }
                        );

                        fileInput.click();

                    };


                    this.subMenu.appendChild(
                        button
                    );


                }
                else if(
                    value &&
                    value.type==="display"
                ){


                    let display =
                    document.createElement(
                        "span"
                    );


                    display.id=
                        value.id ||
                        "";

                    display.className=
                        "ring-id-display";


                    this.subMenu.appendChild(
                        display
                    );


                }
                else if(
                    value &&
                    value.type==="layerStepper"
                ){


                    let minus =
                    document.createElement(
                        "button"
                    );

                    minus.innerText=
                        "LAYER -";

                    minus.onclick=()=>{

                        window.dispatchEvent(
                            new CustomEvent(
                                "layerIndexStep",
                                {detail:{direction:-1}}
                            )
                        );

                    };

                    this.subMenu.appendChild(
                        minus
                    );



                    let display =
                    document.createElement(
                        "span"
                    );

                    display.innerText=
                        this.layerSelection.name;

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
                        "LAYER +";

                    plus.onclick=()=>{

                        window.dispatchEvent(
                            new CustomEvent(
                                "layerIndexStep",
                                {detail:{direction:1}}
                            )
                        );

                    };

                    this.subMenu.appendChild(
                        plus
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




    renderKeyColourPicker(){


        let layerLabel =
        document.createElement(
            "span"
        );

        layerLabel.innerText=
            this.layerSelection.name;

        layerLabel.className=
            "ring-id-display";

        this.subMenu.appendChild(
            layerLabel
        );



        let current =
        document.createElement(
            "span"
        );


        current.innerText=
            "CURRENT";


        current.className=
            "ring-id-display";


        this.styleSwatch(
            current,
            this.keyColour.r,
            this.keyColour.g,
            this.keyColour.b
        );


        this.subMenu.appendChild(
            current
        );



        let pick =
        document.createElement(
            "button"
        );


        pick.innerText=
            "PICK FROM VIDEO";


        pick.onclick=()=>{

            window.dispatchEvent(
                new Event("armKeyColourPicker")
            );

        };


        this.subMenu.appendChild(
            pick
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

                window.dispatchEvent(
                    new CustomEvent(
                        "bodyKeyColour",
                        {
                            detail:{
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




    renderMaskPicker(layerName){


        const state =
            this.maskState[layerName];


        if(
            layerName === "video" ||
            layerName === "body"
        ){

            let layerLabel =
            document.createElement(
                "span"
            );

            layerLabel.innerText=
                this.layerSelection.name;

            layerLabel.className=
                "ring-id-display";

            this.subMenu.appendChild(
                layerLabel
            );

        }


        let sourceMinus =
        document.createElement(
            "button"
        );

        sourceMinus.innerText=
            "SOURCE -";

        sourceMinus.onclick=()=>{

            window.dispatchEvent(
                new CustomEvent(
                    "maskSourceStep",
                    {
                        detail:{
                            layer:layerName,
                            direction:-1
                        }
                    }
                )
            );

        };

        this.subMenu.appendChild(
            sourceMinus
        );



        let sourceDisplay =
        document.createElement(
            "span"
        );

        sourceDisplay.innerText=
            state.sourceLabel;

        sourceDisplay.className=
            "ring-id-display";

        this.subMenu.appendChild(
            sourceDisplay
        );



        let sourcePlus =
        document.createElement(
            "button"
        );

        sourcePlus.innerText=
            "SOURCE +";

        sourcePlus.onclick=()=>{

            window.dispatchEvent(
                new CustomEvent(
                    "maskSourceStep",
                    {
                        detail:{
                            layer:layerName,
                            direction:1
                        }
                    }
                )
            );

        };

        this.subMenu.appendChild(
            sourcePlus
        );




        let channelMinus =
        document.createElement(
            "button"
        );

        channelMinus.innerText=
            "CHANNEL -";

        channelMinus.onclick=()=>{

            window.dispatchEvent(
                new CustomEvent(
                    "maskChannelStep",
                    {
                        detail:{
                            layer:layerName,
                            direction:-1
                        }
                    }
                )
            );

        };

        this.subMenu.appendChild(
            channelMinus
        );



        let channelDisplay =
        document.createElement(
            "span"
        );

        channelDisplay.innerText=
            state.channel.toUpperCase();

        channelDisplay.className=
            "ring-id-display";

        this.subMenu.appendChild(
            channelDisplay
        );



        let channelPlus =
        document.createElement(
            "button"
        );

        channelPlus.innerText=
            "CHANNEL +";

        channelPlus.onclick=()=>{

            window.dispatchEvent(
                new CustomEvent(
                    "maskChannelStep",
                    {
                        detail:{
                            layer:layerName,
                            direction:1
                        }
                    }
                )
            );

        };

        this.subMenu.appendChild(
            channelPlus
        );


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



        if(item==="VIDEO VISIBLE ON/OFF")
            window.dispatchEvent(
                new Event("toggleVideoVisible")
            );



        if(item==="BODY ON/OFF")
            window.dispatchEvent(
                new Event("toggleBody")
            );



        if(item==="BODY VISIBLE ON/OFF")
            window.dispatchEvent(
                new Event("toggleBodyVisible")
            );



        if(item==="THRESHOLD +")
            window.dispatchEvent(
                new Event("thresholdUp")
            );



        if(item==="THRESHOLD -")
            window.dispatchEvent(
                new Event("thresholdDown")
            );



        if(item==="DIFFERENCE/CHROMA")
            window.dispatchEvent(
                new Event("toggleMatteMode")
            );



        if(item==="SOLID/VIDEO")
            window.dispatchEvent(
                new Event("toggleBodyFill")
            );



        if(item==="RINGS ON/OFF")
            window.dispatchEvent(
                new Event("toggleRings")
            );



        if(item==="RINGS VISIBLE ON/OFF")
            window.dispatchEvent(
                new Event("toggleRingsVisible")
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



        if(item==="PLAY/STOP")
            window.dispatchEvent(
                new Event("transportPlayStop")
            );



        if(item==="MINUTE +")
            window.dispatchEvent(
                new Event("transportMinuteUp")
            );



        if(item==="MINUTE -")
            window.dispatchEvent(
                new Event("transportMinuteDown")
            );



        if(item==="SECOND +")
            window.dispatchEvent(
                new Event("transportSecondUp")
            );



        if(item==="SECOND -")
            window.dispatchEvent(
                new Event("transportSecondDown")
            );



        if(item==="FRAME +")
            window.dispatchEvent(
                new Event("transportFrameUp")
            );



        if(item==="FRAME -")
            window.dispatchEvent(
                new Event("transportFrameDown")
            );



        if(item==="SYNC MINUTE +")
            window.dispatchEvent(
                new Event("audioSyncMinuteUp")
            );



        if(item==="SYNC MINUTE -")
            window.dispatchEvent(
                new Event("audioSyncMinuteDown")
            );



        if(item==="SYNC SECOND +")
            window.dispatchEvent(
                new Event("audioSyncSecondUp")
            );



        if(item==="SYNC SECOND -")
            window.dispatchEvent(
                new Event("audioSyncSecondDown")
            );



        if(item==="SYNC FRAME +")
            window.dispatchEvent(
                new Event("audioSyncFrameUp")
            );



        if(item==="SYNC FRAME -")
            window.dispatchEvent(
                new Event("audioSyncFrameDown")
            );



    }

}