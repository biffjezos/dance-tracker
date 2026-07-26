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
        VIDEO and KEY are two independent steppers (video layers+real
        generators vs masks-only), each tracked separately here, kept
        in sync by app.js's layerSelectionChanged event (its "scope"
        field says which one changed). Both render through the same
        renderLayerEditor() - only which selection object it reads
        differs.
        */
        this.videoSelection = {label:"VIDEO 1", kind:"video", visibilityMode:"on"};

        this.maskSelection = {label:"MASK 1", kind:"mask", visibilityMode:"on"};


        window.addEventListener(
            "layerSelectionChanged",
            e=>{

                const selection = {
                    label:e.detail.label,
                    kind:e.detail.kind,
                    visibilityMode:e.detail.visibilityMode,
                    sourceLabel:e.detail.sourceLabel
                };


                if(e.detail.scope === "mask"){

                    this.maskSelection = selection;

                }
                else {

                    this.videoSelection = selection;

                }


                if(e.detail.keyColour){

                    this.keyColour = {
                        r:e.detail.keyColour.r,
                        g:e.detail.keyColour.g,
                        b:e.detail.keyColour.b
                    };

                }


                if(this.path[0] === "video" || this.path[0] === "key"){

                    this.render();

                }

            }
        );


        this.maskState = {

            video:{source:"none", sourceLabel:"NONE", channel:"alpha"},

            mask:{source:"none", sourceLabel:"NONE", channel:"alpha"}

        };


        window.addEventListener(
            "maskSettingsChanged",
            e=>{

                this.maskState[e.detail.scope] = {
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


        this.backgroundState = {

            video:{source:"none", sourceLabel:"NONE", colour:{r:0, g:0, b:0}, blendMode:"normal"},

            mask:{source:"none", sourceLabel:"NONE", colour:{r:0, g:0, b:0}, blendMode:"normal"}

        };


        window.addEventListener(
            "backgroundSettingsChanged",
            e=>{

                this.backgroundState[e.detail.scope] = {
                    source:e.detail.source,
                    sourceLabel:e.detail.sourceLabel,
                    colour:e.detail.colour,
                    blendMode:e.detail.blendMode
                };


                let node =
                    this.node();

                if(
                    node &&
                    node.type==="backgroundPicker"
                ){

                    this.render();

                }

            }
        );


        /*
        No event listener for this one - renderApplyToMaskPicker()
        fetches it fresh synchronously every time it renders (see
        there for why: a broadcast-and-listen round trip here would
        re-enter render() from inside a render()).
        */
        this.applyToMaskState = {

            label:"NONE AVAILABLE",

            options:[]

        };



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
            VIDEO only lists things that really exist: video layers
            you've actually added, plus every rings/ghost/text
            instance you've actually added via GENERATE's ADD RINGS/
            ADD GHOST/ADD TEXT (any number of each, same as video
            layers). Every entry, whatever kind, gets the exact same
            minimal row - VISIBILITY MODE, BACKGROUND, MASK,
            TRANSPORT - plus, for a generator instance, EDIT: a
            bridge to that specific instance's own deep settings
            (ring/ghost/text kinds only - a plain video has nothing
            more to configure beyond the universal row).
            */
            video:{

                type:"layerEditor",

                scope:"video",

                MASK:{

                    type:"maskPicker",

                    scope:"video"

                },

                BACKGROUND:{

                    type:"backgroundPicker",

                    scope:"video"

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

                    "FRAME -":null

                },

                EDIT:{

                    type:"instanceEditor",

                    scope:"video",

                    "RING COLOUR":{

                        type:"ringColourPicker"

                    },

                    CONSTELLATION:[

                        "CONSTELLATION ON/OFF",

                        "DISTANCE +",

                        "DISTANCE -"

                    ],

                    "TEXT COLOUR":colourMenu(
                        "textColour"
                    ),

                    "APPLY TO MASK":{

                        type:"applyToMaskPicker"

                    }

                }

            },



            /*
            KEY is its own selector, scoped to masks only - same
            shape as VIDEO: a minimal top-level row (stepper,
            VISIBILITY, MASK, ADD MASK) plus an EDIT bridge to the
            actual chroma-key deep settings, instead of dumping both
            onto one screen. No BACKGROUND or TRANSPORT here - both
            are video-layer concepts, already set from VIDEO, and
            having a second copy under KEY was confusing (which one's
            real?), not useful.
            */
            key:{

                type:"layerEditor",

                scope:"mask",

                MASK:{

                    type:"maskPicker",

                    scope:"mask"

                },

                EDIT:{

                    type:"instanceEditor",

                    scope:"mask",

                    "KEY COLOUR":{

                        type:"keyColourPicker"

                    },

                    COLOUR:colourMenu(
                        "layerColour"
                    )

                }

            },



            /*
            No default crap - one action per kind, mirrors INPUT's ADD
            VIDEO SOURCE/ADD AUDIO SOURCE exactly. Each click creates a
            brand new instance, which immediately appears as its own
            real entry in VIDEO - that's where it's edited (via EDIT),
            not here. Any number of each, same as video layers.
            */
            generate:{

                "ADD RINGS":null,

                "ADD GHOST":null,

                "ADD TEXT":null

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
            node.type==="backgroundPicker"
        ){

            this.renderBackgroundPicker();

            return;

        }



        if(
            node &&
            node.type==="applyToMaskPicker"
        ){

            this.renderApplyToMaskPicker();

            return;

        }



        if(
            node &&
            node.type==="instanceEditor"
        ){

            this.renderInstanceEditor();

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
    Serves both VIDEO and KEY - which one is which is entirely
    determined by this.node().scope ("video" or "mask"). Both show
    the exact same minimal row: LAYER -/+ stepper, VISIBILITY MODE,
    BACKGROUND COLOUR, MASK, TRANSPORT. KEY additionally shows the
    actual chroma-key deep settings, since scope is always "mask"
    there and a mask has no other screen to live on.
    */
    renderLayerEditor(){


        const scope =
            this.node().scope;

        const selection =
            scope === "mask"
            ?
            this.maskSelection
            :
            this.videoSelection;


        /*
        selection.label is null when nothing real exists in this
        scope yet (no video source ever added/camera never turned on,
        or no mask ever added) - same "doesn't exist until created"
        rule as everything else in this app. Nothing to step through
        or operate on, so show that plainly instead of a stepper
        pointed at a node that was never created. KEY still offers
        ADD MASK here (masks can be added before any video exists -
        they just won't have a real SOURCE to key yet).
        */
        if(!selection.label){

            let empty =
            document.createElement(
                "span"
            );

            empty.innerText=
                scope === "mask"
                ?
                "NO MASKS YET"
                :
                "NO VIDEO SOURCES YET - SEE INPUT";

            empty.className=
                "ring-id-display";

            this.subMenu.appendChild(
                empty
            );


            if(scope === "mask"){

                this.addLayerButton(
                    "ADD MASK",
                    "addMaskLayer"
                );

            }


            return;

        }


        let minus =
        document.createElement(
            "button"
        );

        minus.innerText=
            "LAYER -";

        minus.onclick=()=>{

            window.dispatchEvent(
                new CustomEvent(
                    scope === "mask" ? "maskIndexStep" : "videoIndexStep",
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
            selection.label;

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
                    scope === "mask" ? "maskIndexStep" : "videoIndexStep",
                    {detail:{direction:1}}
                )
            );

        };

        this.subMenu.appendChild(
            plus
        );



        /*
        One button, four states, cycled forward each click. Re-renders
        immediately after dispatching since, unlike the other buttons
        here, this one's own label depends on the state it just
        changed (dispatchEvent runs the app.js handler synchronously,
        so the selection is already updated by the time render()
        reads it back).
        */
        let visibility =
        document.createElement(
            "button"
        );

        visibility.innerText=
            "VISIBILITY: " +
            (
                VISIBILITY_MODE_LABELS[
                    selection.visibilityMode
                ] ||
                "ON"
            );

        visibility.onclick=()=>{

            window.dispatchEvent(
                new CustomEvent(
                    "cycleVisibilityMode",
                    {detail:{scope:scope}}
                )
            );

            this.render();

        };

        this.subMenu.appendChild(
            visibility
        );



        if(scope === "video"){

            this.addLayerScreenButton(
                "BACKGROUND",
                "BACKGROUND"
            );

            this.addLayerScreenButton(
                "TRANSPORT",
                "TRANSPORT"
            );

        }


        this.addLayerScreenButton(
            "MASKED BY",
            "MASK"
        );



        if(scope === "mask"){

            this.addLayerButton(
                "ADD MASK",
                "addMaskLayer"
            );

        }



        if(
            (
                scope === "video" &&
                (
                    selection.kind === "rings" ||
                    selection.kind === "ghost" ||
                    selection.kind === "text"
                )
            )
            ||
            scope === "mask"
        ){

            this.addLayerScreenButton(
                "EDIT",
                "EDIT"
            );

        }


    }




    /*
    Reached via EDIT from either VIDEO or KEY (this.node().scope says
    which) - the type-specific deep settings for whichever thing is
    currently selected there. No stepper of its own, no universal row
    duplicated here - just what's unique to that kind, operating on
    whichever instance the calling scope has selected.
    */
    renderInstanceEditor(){


        const scope =
            this.node().scope || "video";

        const kind =
            scope === "mask"
            ?
            this.maskSelection.kind
            :
            this.videoSelection.kind;


        if(kind === "rings"){

            this.addLayerButton(
                "RING COUNT +",
                "ringCountUp"
            );

            this.addLayerButton(
                "RING COUNT -",
                "ringCountDown"
            );

            this.addLayerButton(
                "RING SIZE +",
                "ringSizeUp"
            );

            this.addLayerButton(
                "RING SIZE -",
                "ringSizeDown"
            );

            this.addLayerButton(
                "RING THICKNESS +",
                "ringThicknessUp"
            );

            this.addLayerButton(
                "RING THICKNESS -",
                "ringThicknessDown"
            );

            this.addLayerScreenButton(
                "COLOUR",
                "RING COLOUR"
            );

            this.addLayerScreenButton(
                "CONSTELLATION",
                "CONSTELLATION"
            );

        }
        else if(kind === "ghost"){

            this.addLayerButton(
                "GHOST +",
                "ghostUp"
            );

            this.addLayerButton(
                "GHOST -",
                "ghostDown"
            );

            this.addLayerButton(
                "GHOST DELAY +",
                "ghostDelayUp"
            );

            this.addLayerButton(
                "GHOST DELAY -",
                "ghostDelayDown"
            );

            this.addLayerScreenButton(
                "APPLY TO MASK",
                "APPLY TO MASK"
            );

        }
        else if(kind === "text"){

            let input =
            document.createElement(
                "input"
            );

            input.type="text";

            input.placeholder=
                "ENTER YOUR TEXT HERE";

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
                            "setText",
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


            this.addLayerButton(
                "SIZE +",
                "textSizeUp"
            );

            this.addLayerButton(
                "SIZE -",
                "textSizeDown"
            );

            this.addLayerScreenButton(
                "COLOUR",
                "TEXT COLOUR"
            );

        }
        else if(
            kind === "mask" ||
            kind === "standaloneMask"
        ){

            if(kind === "standaloneMask"){

                let sourceMinus =
                document.createElement(
                    "button"
                );

                sourceMinus.innerText=
                    "SOURCE -";

                sourceMinus.onclick=()=>{

                    window.dispatchEvent(
                        new CustomEvent(
                            "maskVideoSourceStep",
                            {detail:{direction:-1}}
                        )
                    );

                    this.render();

                };

                this.subMenu.appendChild(
                    sourceMinus
                );



                let sourceDisplay =
                document.createElement(
                    "span"
                );

                sourceDisplay.innerText=
                    "SOURCE: " +
                    (this.maskSelection.sourceLabel || "NONE");

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
                            "maskVideoSourceStep",
                            {detail:{direction:1}}
                        )
                    );

                    this.render();

                };

                this.subMenu.appendChild(
                    sourcePlus
                );

            }


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
            this.maskSelection.label;

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


        const scope =
            this.node().scope;

        const state =
            this.maskState[scope];

        const selection =
            scope === "mask"
            ?
            this.maskSelection
            :
            this.videoSelection;


        let layerLabel =
        document.createElement(
            "span"
        );

        layerLabel.innerText=
            selection.label;

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
                            direction:-1,
                            scope:scope
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
                            direction:1,
                            scope:scope
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
                            direction:-1,
                            scope:scope
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
                            direction:1,
                            scope:scope
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
    Shares the SOURCE stepper shape with MASK, but steps through
    NONE/COLOUR/every real layer instead of NONE/BACKGROUND/every
    real layer, and shows swatches (instead of a channel stepper)
    only when COLOUR is the current source - that's the only case
    where a colour is actually used for anything.
    */
    renderBackgroundPicker(){


        const scope =
            this.node().scope;

        const state =
            this.backgroundState[scope];

        const selection =
            scope === "mask"
            ?
            this.maskSelection
            :
            this.videoSelection;


        let layerLabel =
        document.createElement(
            "span"
        );

        layerLabel.innerText=
            selection.label;

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
                    "backgroundSourceStep",
                    {
                        detail:{
                            direction:-1,
                            scope:scope
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
                    "backgroundSourceStep",
                    {
                        detail:{
                            direction:1,
                            scope:scope
                        }
                    }
                )
            );

        };

        this.subMenu.appendChild(
            sourcePlus
        );



        if(state.source === "colour"){

            let blendMinus =
            document.createElement(
                "button"
            );

            blendMinus.innerText=
                "BLEND -";

            blendMinus.onclick=()=>{

                window.dispatchEvent(
                    new CustomEvent(
                        "backgroundBlendModeStep",
                        {
                            detail:{
                                direction:-1,
                                scope:scope
                            }
                        }
                    )
                );

            };

            this.subMenu.appendChild(
                blendMinus
            );



            let blendDisplay =
            document.createElement(
                "span"
            );

            blendDisplay.innerText=
                "BLEND: " +
                (state.blendMode || "normal").toUpperCase();

            blendDisplay.className=
                "ring-id-display";

            this.subMenu.appendChild(
                blendDisplay
            );



            let blendPlus =
            document.createElement(
                "button"
            );

            blendPlus.innerText=
                "BLEND +";

            blendPlus.onclick=()=>{

                window.dispatchEvent(
                    new CustomEvent(
                        "backgroundBlendModeStep",
                        {
                            detail:{
                                direction:1,
                                scope:scope
                            }
                        }
                    )
                );

            };

            this.subMenu.appendChild(
                blendPlus
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
                            "layerBackgroundColour",
                            {
                                detail:{
                                    scope:scope,
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


    }




    /*
    Ghost's own effect input - which mask's shape feeds the trail
    history. A direct selector, not a stepper: one button per mask
    currently in MASK WHITE visibility mode, no NONE fallback
    (app.js's eligibleMaskTargets enforces that - the list is simply
    empty if nothing qualifies yet). Different from MASKED BY, which
    masks Ghost's composited output and lives under VIDEO instead.

    Fetches fresh data every time it renders, synchronously, via a
    request object app.js mutates in place rather than a broadcast
    event - dispatching a broadcast from here and having its listener
    call render() would re-enter this same method from inside itself.
    */
    renderApplyToMaskPicker(){


        const request = {
            label:null,
            options:null
        };

        window.dispatchEvent(
            new CustomEvent(
                "requestApplyToMaskRefresh",
                {detail:request}
            )
        );

        if(request.options){

            this.applyToMaskState = {
                label:request.label,
                options:request.options
            };

        }


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

                this.render();

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



        if(item==="ADD RINGS")
            window.dispatchEvent(
                new Event("addRingsLayer")
            );



        if(item==="ADD GHOST")
            window.dispatchEvent(
                new Event("addGhostLayer")
            );



        if(item==="ADD TEXT")
            window.dispatchEvent(
                new Event("addTextLayer")
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
                new Event("toggleRingsEnabled")
            );



        if(item==="CONSTELLATION ON/OFF")
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
