/*
==================================================
DANCE TRACKER 5000
AMIGA TWO ROW MENU SYSTEM
==================================================
*/


const VISIBILITY_MODE_LABELS = {
    on:"ON",
    alpha:"ALPHA",
    maskWhite:"MASK WHITE",
    off:"OFF"
};


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


        /*
        Every kind of layer (video, mask, rings, text, ghost) is
        navigated through the exact same LAYER -/+ stepper and shows
        the exact same shape of universal controls (see
        renderLayerEditor) - this mirror just tracks which one is
        currently selected, what kind it is, and its visibility mode,
        kept in sync by app.js's layerSelectionChanged event.
        */
        this.registrySelection = {label:"LAYER 1", kind:"video", visibilityMode:"on"};


        window.addEventListener(
            "layerSelectionChanged",
            e=>{

                this.registrySelection = {
                    label:e.detail.label,
                    kind:e.detail.kind,
                    visibilityMode:e.detail.visibilityMode
                };

                if(e.detail.keyColour){

                    this.keyColour = {
                        r:e.detail.keyColour.r,
                        g:e.detail.keyColour.g,
                        b:e.detail.keyColour.b
                    };

                }


                if(this.path[0] === "video"){

                    this.render();

                }

            }
        );


        this.maskState = {

            source:"none",

            sourceLabel:"NONE",

            channel:"alpha"

        };


        window.addEventListener(
            "maskSettingsChanged",
            e=>{

                this.maskState = {
                    source:e.detail.source,
                    sourceLabel:e.detail.sourceLabel,
                    channel:e.detail.channel
                };


                let node =
                    this.node();

                if(
                    node &&
                    node.type==="maskPicker"
                ){

                    this.render();

                }

            }
        );


        this.applyToMaskState = {

            label:"NONE AVAILABLE",

            options:[]

        };


        window.addEventListener(
            "applyToMaskChanged",
            e=>{

                this.applyToMaskState = {
                    label:e.detail.label,
                    options:e.detail.options
                };


                let node =
                    this.node();

                if(
                    node &&
                    node.type==="applyToMaskPicker"
                ){

                    this.render();

                }

            }
        );



        this.menus = {


            /*
            One unified add per media type - no separate "load into
            the camera slot" vs "add a layer" anymore. Camera keeps
            its own on/off since it's not a file. TRANSPORT moved out
            to VIDEO (see below, "similar to BACKGROUND or MASK" -
            reachable per selected node, not a global INPUT thing) -
            interim: still the one shared transport tree underneath,
            full per-node offset overhaul is tracked separately.
            */
            input:{

                CAMERA:{

                    "CAMERA ON/OFF":null

                },

                "ADD VIDEO SOURCE":{

                    type:"fileInput",

                    accept:"video/*",

                    event:"addVideoLayer"

                },

                "ADD AUDIO SOURCE":{

                    type:"fileInput",

                    accept:"audio/*",

                    event:"loadAudioFile"

                }

            },



            /*
            The universal navigator: every video feed, every derived
            mask, and the rings/ghost/text generators, all through the
            same LAYER -/+ stepper, all showing the same universal
            row (VISIBILITY MODE/BACKGROUND COLOUR/MASKED BY/
            TRANSPORT) - see renderLayerEditor(). Mask-kind entries
            additionally get their own deep settings here since masks
            have no other screen; rings/ghost/text's deep settings
            live under GENERATE instead, not duplicated here.
            */
            video:{

                type:"layerEditor",

                "MASKED BY":{

                    type:"maskPicker"

                },

                "KEY COLOUR":{

                    type:"keyColourPicker"

                },

                COLOUR:colourMenu(
                    "layerColour"
                ),

                "BACKGROUND COLOUR":colourMenu(
                    "videoBackgroundColour"
                ),

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

                    "FRAME -":null

                }

            },



            /*
            No default crap - just the three fixed generators, no add
            action, no stepper. Each shows only what's specific to
            that generator; visibility/background/masked by for these
            live under VIDEO instead.
            */
            generate:{

                GHOST:{

                    "GHOST +":null,

                    "GHOST -":null,

                    "GHOST DELAY +":null,

                    "GHOST DELAY -":null,

                    "APPLY TO MASK":{

                        type:"applyToMaskPicker"

                    }

                },

                RINGS:{

                    "RING COUNT +":null,

                    "RING COUNT -":null,

                    "RING SIZE +":null,

                    "RING SIZE -":null,

                    "RING THICKNESS +":null,

                    "RING THICKNESS -":null,

                    COLOUR:{

                        type:"ringColourPicker"

                    },

                    CONSTELLATION:[

                        "ON/OFF",

                        "DISTANCE +",

                        "DISTANCE -"

                    ]

                },

                TEXT:{

                    CONTENT:{

                        type:"input",

                        placeholder:"ENTER YOUR TEXT HERE",

                        event:"setText"

                    },

                    "TEXT SIZE +":null,

                    "TEXT SIZE -":null,

                    COLOUR:colourMenu(
                        "textColour"
                    )

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

                },

                BACKGROUND:{

                    COLOUR:colourMenu(
                        "videoBackgroundColour"
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
            node.type==="layerEditor"
        ){

            this.renderLayerEditor();

            return;

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

            this.renderMaskPicker();

            return;

        }



        if(
            node &&
            node.type==="applyToMaskPicker"
        ){

            this.renderApplyToMaskPicker();

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




    /*
    The universal navigator screen: LAYER -/+ stepper, then
    VISIBILITY MODE, BACKGROUND COLOUR, MASKED BY, TRANSPORT -
    identical for every kind, video/mask/rings/ghost/text alike. Mask
    kind additionally gets its own deep settings since a mask has no
    other screen - everything else's deep settings live under
    GENERATE instead, not duplicated here.
    */
    renderLayerEditor(){


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
            this.registrySelection.label;

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



        /*
        One button, four states, cycled forward each click - replaces
        the old separate ON/OFF + VISIBLE ON/OFF pair. Re-renders
        immediately after dispatching since, unlike the other
        buttons here, this one's own label depends on the state it
        just changed (dispatchEvent runs the app.js handler
        synchronously, so registrySelection is already updated by
        the time render() reads it back).
        */
        let visibility =
        document.createElement(
            "button"
        );

        visibility.innerText=
            "VISIBILITY: " +
            (
                VISIBILITY_MODE_LABELS[
                    this.registrySelection.visibilityMode
                ] ||
                "ON"
            );

        visibility.onclick=()=>{

            window.dispatchEvent(
                new Event("cycleVisibilityMode")
            );

            this.render();

        };

        this.subMenu.appendChild(
            visibility
        );



        this.addLayerScreenButton(
            "BACKGROUND COLOUR",
            "BACKGROUND COLOUR"
        );


        let maskedBy =
        document.createElement(
            "button"
        );

        maskedBy.innerText=
            "MASKED BY";

        maskedBy.onclick=()=>{

            this.enter(
                "MASKED BY"
            );

        };

        this.subMenu.appendChild(
            maskedBy
        );



        this.addLayerScreenButton(
            "TRANSPORT",
            "TRANSPORT"
        );



        const kind =
            this.registrySelection.kind;


        if(kind === "mask"){

            this.addLayerButton(
                "CAPTURE BACKGROUND",
                "captureLayerBackground"
            );

            this.addLayerButton(
                "THRESHOLD +",
                "thresholdUp"
            );

            this.addLayerButton(
                "THRESHOLD -",
                "thresholdDown"
            );

            this.addLayerButton(
                "DIFFERENCE/CHROMA",
                "toggleMatteMode"
            );

            this.addLayerButton(
                "SOLID/VIDEO",
                "toggleLayerFill"
            );

            this.addLayerScreenButton(
                "COLOUR",
                "COLOUR"
            );

            this.addLayerScreenButton(
                "KEY COLOUR",
                "KEY COLOUR"
            );

        }


    }




    addLayerButton(label, eventName){


        let button =
        document.createElement(
            "button"
        );

        button.innerText=
            label;

        button.onclick=()=>{

            console.log(
                "MENU SELECTED:",
                label
            );

            window.dispatchEvent(
                new Event(eventName)
            );

        };

        this.subMenu.appendChild(
            button
        );


    }




    addLayerScreenButton(label, screenKey){


        let button =
        document.createElement(
            "button"
        );

        button.innerText=
            label;

        button.onclick=()=>{

            this.enter(
                screenKey
            );

        };

        this.subMenu.appendChild(
            button
        );


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
            this.registrySelection.label;

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




    renderMaskPicker(){


        const state =
            this.maskState;


        let layerLabel =
        document.createElement(
            "span"
        );

        layerLabel.innerText=
            this.registrySelection.label;

        layerLabel.className=
            "ring-id-display";

        this.subMenu.appendChild(
            layerLabel
        );


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




    /*
    Ghost's own effect input - which mask's shape feeds the trail
    history. A direct selector, not a stepper: one button per mask
    currently in MASK WHITE visibility mode, no NONE fallback
    (app.js's eligibleMaskTargets enforces that - the list is simply
    empty if nothing qualifies yet). Different from MASKED BY, which
    masks Ghost's composited output and lives under VIDEO instead.
    */
    renderApplyToMaskPicker(){


        let current =
        document.createElement(
            "span"
        );

        current.innerText=
            "CURRENT: " +
            this.applyToMaskState.label;

        current.className=
            "ring-id-display";

        this.subMenu.appendChild(
            current
        );



        this.applyToMaskState.options.forEach(option=>{


            let button =
            document.createElement(
                "button"
            );

            button.innerText=
                option.label;

            button.onclick=()=>{

                window.dispatchEvent(
                    new CustomEvent(
                        "setApplyToMask",
                        {
                            detail:{
                                id:option.id
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



        if(item==="ADD RINGS LAYER")
            window.dispatchEvent(
                new Event("addRingsLayer")
            );



        if(item==="RECORD ON/OFF")
            window.dispatchEvent(
                new Event("toggleRecord")
            );



        if(item==="ON/OFF")
            window.dispatchEvent(
                new Event("toggleConstellation")
            );



        if(item==="DISTANCE +")
            window.dispatchEvent(
                new Event("constellationDistanceUp")
            );



        if(item==="DISTANCE -")
            window.dispatchEvent(
                new Event("constellationDistanceDown")
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



        if(item==="GHOST +")
            window.dispatchEvent(
                new Event("ghostUp")
            );



        if(item==="GHOST -")
            window.dispatchEvent(
                new Event("ghostDown")
            );



        if(item==="GHOST DELAY +")
            window.dispatchEvent(
                new Event("ghostDelayUp")
            );



        if(item==="GHOST DELAY -")
            window.dispatchEvent(
                new Event("ghostDelayDown")
            );



        if(item==="TEXT SIZE +")
            window.dispatchEvent(
                new Event("textSizeUp")
            );



        if(item==="TEXT SIZE -")
            window.dispatchEvent(
                new Event("textSizeDown")
            );



    }

}
