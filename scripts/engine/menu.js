/*
==================================================
DANCE TRACKER 5000
AMIGA TWO ROW MENU SYSTEM
==================================================
*/
const VISIBILITY_MODE_LABELS = {
    on: "ON",
    alpha: "ALPHA",
    maskWhite: "MASK WHITE",
    off: "OFF"
};
const SWATCHES = [{
    label: "BLACK",
    r: 0,
    g: 0,
    b: 0
}, {
    label: "WHITE",
    r: 255,
    g: 255,
    b: 255
}, {
    label: "RED",
    r: 255,
    g: 0,
    b: 0
}, {
    label: "GREEN",
    r: 0,
    g: 255,
    b: 0
}, {
    label: "BLUE",
    r: 0,
    g: 150,
    b: 255
}, {
    label: "MAGENTA",
    r: 255,
    g: 0,
    b: 255
}, {
    label: "CYAN",
    r: 0,
    g: 255,
    b: 255
}, {
    label: "YELLOW",
    r: 255,
    g: 255,
    b: 0
}, {
    label: "DARK GREEN",
    r: 0,
    g: 20,
    b: 0
}, {
    label: "DARK BLUE",
    r: 0,
    g: 10,
    b: 30
}];

function colourMenu(event) {
    return SWATCHES.map(swatch => ({
        label: swatch.label,
        event: event,
        r: swatch.r,
        g: swatch.g,
        b: swatch.b
    }));
}
export class MenuManager {
    constructor() {
        this.subMenu = document.getElementById("sub-menu");
        this.path = [];
        this.textValue = "";
        this.ringId = 1;
        this.keyColour = {
            r: 0,
            g: 255,
            b: 0
        };
        window.addEventListener("bodyKeyColour", e => {
            this.keyColour = {
                r: e.detail.r,
                g: e.detail.g,
                b: e.detail.b
            };
            let node = this.node();
            if (node && node.type === "keyColourPicker") {
                this.render();
            }
        });
        /*
        VIDEO, KEY and COMPOSE are three independent steppers (video
        layers+real generators, masks-only, composites-only), each
        tracked separately here, kept in sync by app.js's
        layerSelectionChanged event (its "scope" field says which one
        changed). VIDEO and KEY render through the same
        renderLayerEditor() - only which selection object it reads
        differs; COMPOSE has its own renderComposePicker().

        label:null until the first real layerSelectionChanged event -
        renderLayerEditor()/renderComposePicker() read that as "nothing
        real exists yet" and show an honest empty state. Never seed
        this with a fake "VIDEO 1"/"MASK 1"/"COMPOSITE 1" placeholder -
        that displays as a real, selectable node before the user has
        created anything.
        */
        this.videoSelection = {
            label: null,
            kind: null,
            visibilityMode: "on"
        };
        this.maskSelection = {
            label: null,
            kind: null,
            visibilityMode: "on"
        };
        this.compositeSelection = {
            label: null,
            visibilityMode: "on"
        };
        window.addEventListener("layerSelectionChanged", e => {
            const selection = {
                id: e.detail.id,
                label: e.detail.label,
                kind: e.detail.kind,
                visibilityMode: e.detail.visibilityMode,
                sourceLabel: e.detail.sourceLabel,
                mode: e.detail.mode,
                ringCount: e.detail.ringCount,
                foregroundLabel: e.detail.foregroundLabel,
                backgroundLabel: e.detail.backgroundLabel,
                blendMode: e.detail.blendMode
            };
            if (e.detail.scope === "mask") {
                this.maskSelection = selection;
            } else if (e.detail.scope === "composite") {
                this.compositeSelection = selection;
            } else {
                this.videoSelection = selection;
            }
            if (e.detail.keyColour) {
                this.keyColour = {
                    r: e.detail.keyColour.r,
                    g: e.detail.keyColour.g,
                    b: e.detail.keyColour.b
                };
            }
            if (this.path[0] === "video" || this.path[0] === "key" || this.path[0] === "compose") {
                this.render();
            }
        });
        this.maskState = {
            video: {
                source: "none",
                sourceLabel: "NONE",
                channel: "alpha"
            },
            mask: {
                source: "none",
                sourceLabel: "NONE",
                channel: "alpha"
            }
        };
        window.addEventListener("maskSettingsChanged", e => {
            this.maskState[e.detail.scope] = {
                source: e.detail.source,
                sourceLabel: e.detail.sourceLabel,
                channel: e.detail.channel
            };
            let node = this.node();
            if (node && node.type === "maskPicker") {
                this.render();
            }
        });
        /*
        No event listener for this one - renderApplyToMaskPicker()
        fetches it fresh synchronously every time it renders (see
        there for why: a broadcast-and-listen round trip here would
        re-enter render() from inside a render()).
        */
        this.applyToMaskState = {
            label: "NONE AVAILABLE",
            options: []
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
            input: {
                CAMERA: {
                    "CAMERA ON/OFF": null
                },
                "ADD VIDEO SOURCE": {
                    type: "fileInput",
                    accept: "video/*",
                    event: "addVideoLayer"
                }
            },
            /*
            NODES only lists things that really exist: video layers
            you've actually added, every rings/ghost/text instance
            you've actually added via GENERATE's ADD RINGS/ADD GHOST/
            ADD TEXT, every mask you've actually added via KEY's ADD
            MASK, and every composite you've actually added via
            COMPOSE's ADD (any number of each, same list, no defaults).
            Every entry, whatever kind, gets the exact same minimal
            row - VISIBILITY MODE, MASKED BY, TRANSPORT - plus EDIT for
            anything with its own deep settings (rings/ghost/text/mask
            kinds - a plain video has nothing more to configure beyond
            the universal row, and a composite's own settings live on
            COMPOSE's screen, not behind EDIT here).
            */
            video: {},
            /*
            KEY is its own selector, scoped to masks only - a filtered
            view of the exact same mask entries NODES already lists
            (see getMaskRegistry() in app.js), same shape as NODES: a
            minimal top-level row (stepper, VISIBILITY, MASK, ADD
            MASK) plus an EDIT bridge to the actual chroma-key deep
            settings, instead of dumping both onto one screen. No
            TRANSPORT button here specifically - not because masks
            lack one (they don't), but because it's already reachable
            for these same entries from NODES, and a second copy under
            KEY was confusing (which one's real?), not useful.
            */
            key: {},
            /*
            No default crap - one action per kind, mirrors INPUT's ADD
            VIDEO SOURCE/ADD AUDIO SOURCE exactly. Each click creates a
            brand new instance, which immediately appears as its own
            real entry in VIDEO - that's where it's edited (via EDIT),
            not here. Any number of each, same as video layers.
            */
            generate: {},
            animations: {},
            /*
            The only place two nodes get drawn together - see
            CLAUDE.md. Not a "layerEditor" like NODES/KEY: a COMPOSITE
            has no deeper settings behind an EDIT bridge, so its own
            stepper and its foreground/background/blend controls all
            render on this one screen (renderComposePicker).
            */
            compose: {},
            transform: {},
            output: {}
        };
    }
    init() {
        document.querySelectorAll(".main-menu button").forEach(button => {
            button.addEventListener("click",
                () => {
                    this.show(button.dataset.menu);
                });
        });
        this.show("input");
    }
    show(menuName) {
        this.path = [menuName];
        this.render();
    }
    enter(category) {
        this.path.push(category);
        this.render();
    }
    up() {
        if (this.path.length > 1) {
            this.path.pop();
            this.render();
        }
    }
    node() {
        let node = this.menus;
        this.path.forEach(key => {
            node = node[key];
        });
        return node;
    }
    render() {
        this.subMenu.innerHTML = "";
        let node = this.node();
        if (this.path.length > 1) {
            let up = document.createElement("button");
            up.innerText = "UP";
            up.className = "up-button";
            up.onclick = () => {
                this.up();
            };
            this.subMenu.appendChild(up);
        }
        if (node && node.type === "layerEditor") {
            this.renderLayerEditor();
            return;
        }
        if (node && node.type === "ringColourPicker") {
            this.renderRingColourPicker();
            return;
        }
        if (node && node.type === "keyColourPicker") {
            this.renderKeyColourPicker();
            return;
        }
        if (node && node.type === "maskPicker") {
            this.renderMaskPicker();
            return;
        }
        if (node && node.type === "composePicker") {
            this.renderComposePicker();
            return;
        }
        if (node && node.type === "applyToMaskPicker") {
            this.renderApplyToMaskPicker();
            return;
        }
        if (node && node.type === "instanceEditor") {
            this.renderInstanceEditor();
            return;
        }
        if (Array.isArray(node)) {
            node.forEach(item => {
                let button = document.createElement("button");
                if (typeof item === "object") {
                    button.innerText = item.label;
                    this.styleSwatch(button, item.r, item.g, item.b);
                    button.onclick = () => {
                        console.log("MENU SELECTED:", item.label);
                        window.dispatchEvent(new CustomEvent(item.event, {
                            detail: {
                                r: item.r,
                                g: item.g,
                                b: item.b
                            }
                        }));
                    };
                } else {
                    button.innerText = item;
                    button.onclick = () => {
                        this.select(item);
                    };
                }
                this.subMenu.appendChild(button);
            });
        } else {
            Object.keys(node).forEach(key => {
                let value = node[key];
                if (value && value.type === "input") {
                    let input = document.createElement("input");
                    input.type = "text";
                    input.placeholder = value.placeholder || "";
                    input.maxLength = 200;
                    input.className = "menu-input";
                    input.value = this.textValue;
                    input.addEventListener("input",
                        () => {
                            this.textValue = input.value;
                            window.dispatchEvent(new CustomEvent(value.event, {
                                detail: {
                                    value: input.value
                                }
                            }));
                        });
                    this.subMenu.appendChild(input);
                } else if (value && value.type === "fileInput") {
                    let button = document.createElement("button");
                    button.innerText = key;
                    button.onclick = () => {
                        let fileInput = document.createElement("input");
                        fileInput.type = "file";
                        fileInput.accept = value.accept || "";
                        fileInput.addEventListener("change",
                            () => {
                                if (fileInput.files.length === 0) return;
                                console.log("MENU SELECTED:", key);
                                window.dispatchEvent(new CustomEvent(value.event, {
                                    detail: {
                                        file: fileInput.files[0]
                                    }
                                }));
                            });
                        fileInput.click();
                    };
                    this.subMenu.appendChild(button);
                } else if (value && value.type === "display") {
                    let display = document.createElement("span");
                    display.id = value.id || "";
                    display.className = "ring-id-display";
                    this.subMenu.appendChild(display);
                } else if (value && typeof value === "object") {
                    let button = document.createElement("button");
                    button.innerText = key;
                    button.onclick = () => {
                        this.enter(key);
                    };
                    this.subMenu.appendChild(button);
                } else {
                    let button = document.createElement("button");
                    button.innerText = key;
                    button.onclick = () => {
                        this.select(key);
                    };
                    this.subMenu.appendChild(button);
                }
            });
        }
    }
    styleSwatch(button, r, g, b) {
        button.style.backgroundColor = "rgb(" + r + "," + g + "," + b + ")";
        const brightness = (r * 299 + g * 587 + b * 114) / 1000;
        button.style.color = brightness > 150 ? "#000" : "#fff";
    }
    /*
    Serves both VIDEO and KEY - which one is which is entirely
    determined by this.node().scope ("video" or "mask"). Both show
    the exact same minimal row: NODE -/+ stepper, VISIBILITY MODE,
    MASKED BY, TRANSPORT (video only). KEY additionally shows the
    actual chroma-key deep settings, since scope is always "mask"
    there and a mask has no other screen to live on.
    */
    renderLayerEditor() {
        const scope = this.node().scope;
        const selection = scope === "mask" ? this.maskSelection : this.videoSelection;
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
        if (!selection.label) {
            let empty = document.createElement("span");
            empty.innerText = scope === "mask" ? "NO MASKS YET" : "NO NODES YET - SEE INPUT/KEY/GENERATE";
            empty.className = "ring-id-display";
            this.subMenu.appendChild(empty);
            if (scope === "mask") {
                this.addLayerButton("ADD MASK", "addMaskLayer");
            }
            return;
        }
        let minus = document.createElement("button");
        minus.innerText = "NODE -";
        minus.onclick = () => {
            window.dispatchEvent(new CustomEvent(scope === "mask" ? "maskIndexStep" : "videoIndexStep", {
                detail: {
                    direction: -1
                }
            }));
        };
        this.subMenu.appendChild(minus);
        let display = document.createElement("span");
        display.innerText = selection.label;
        display.className = "ring-id-display";
        this.subMenu.appendChild(display);
        let plus = document.createElement("button");
        plus.innerText = "NODE +";
        plus.onclick = () => {
            window.dispatchEvent(new CustomEvent(scope === "mask" ? "maskIndexStep" : "videoIndexStep", {
                detail: {
                    direction: 1
                }
            }));
        };
        this.subMenu.appendChild(plus);
        /*
        One button, four states, cycled forward each click. Re-renders
        immediately after dispatching since, unlike the other buttons
        here, this one's own label depends on the state it just
        changed (dispatchEvent runs the app.js handler synchronously,
        so the selection is already updated by the time render()
        reads it back).
        */
        let visibility = document.createElement("button");
        visibility.innerText = "VISIBILITY: " + (VISIBILITY_MODE_LABELS[selection.visibilityMode] || "ON");
        visibility.onclick = () => {
            window.dispatchEvent(new CustomEvent("cycleVisibilityMode", {
                detail: {
                    scope: scope
                }
            }));
            this.render();
        };
        this.subMenu.appendChild(visibility);
        if (scope === "video") {
            this.addLayerScreenButton("MASKED BY", "MASK");
            this.addLayerScreenButton("TRANSPORT", "TRANSPORT");
        }
        if (scope === "video" && selection.kind === "composite") {
            /*
            A composite's settings live on COMPOSE's own screen, not
            behind a local EDIT bridge - COMPOSE has its own
            independent stepper (this.compositeSelection), so EDIT has
            to point it at this exact composite (focusComposite) before
            jumping there, otherwise COMPOSE could open on a different
            one than the user just selected here.
            */
            let editButton = document.createElement("button");
            editButton.innerText = "EDIT";
            editButton.onclick = () => {
                window.dispatchEvent(new CustomEvent("focusComposite", {
                    detail: {
                        id: selection.id
                    }
                }));
                this.show("compose");
            };
            this.subMenu.appendChild(editButton);
        } else if (
            (scope === "video" && (selection.kind === "rings" || selection.kind === "ghost" || selection.kind ===
                "text" || selection.kind === "standaloneMask")) || scope === "mask") {
            this.addLayerScreenButton("EDIT", "EDIT");
        }
        if (scope === "mask") {
            this.addLayerButton("ADD MASK", "addMaskLayer");
        }
    }
    /*
    Reached via EDIT from either VIDEO or KEY (this.node().scope says
    which) - the type-specific deep settings for whichever thing is
    currently selected there. No stepper of its own, no universal row
    duplicated here - just what's unique to that kind, operating on
    whichever instance the calling scope has selected.
    */
    renderInstanceEditor() {
        const scope = this.node().scope || "video";
        const kind = scope === "mask" ? this.maskSelection.kind : this.videoSelection.kind;
        if (kind === "rings") {
            this.addLayerButton("COUNT +", "ringCountUp");
            this.addLayerButton("COUNT -", "ringCountDown");
            this.addLayerButton("SIZE +", "ringSizeUp");
            this.addLayerButton("SIZE -", "ringSizeDown");
            this.addLayerButton("THICKNESS +", "ringThicknessUp");
            this.addLayerButton("THICKNESS -", "ringThicknessDown");
            this.addLayerScreenButton("COLOUR", "RING COLOUR");
            this.addLayerScreenButton("CONSTELLATION", "CONSTELLATION");
        } else if (kind === "ghost") {
            this.addLayerButton("GHOST +", "ghostUp");
            this.addLayerButton("GHOST -", "ghostDown");
            this.addLayerButton("GHOST DELAY +", "ghostDelayUp");
            this.addLayerButton("GHOST DELAY -", "ghostDelayDown");
            this.addLayerScreenButton("APPLY TO MASK", "APPLY TO MASK");
        } else if (kind === "text") {
            let input = document.createElement("input");
            input.type = "text";
            input.placeholder = "ENTER YOUR TEXT HERE";
            input.maxLength = 200;
            input.className = "menu-input";
            input.value = this.textValue;
            input.addEventListener("input",
                () => {
                    this.textValue = input.value;
                    window.dispatchEvent(new CustomEvent("setText", {
                        detail: {
                            value: input.value
                        }
                    }));
                });
            this.subMenu.appendChild(input);
            this.addLayerButton("SIZE +", "textSizeUp");
            this.addLayerButton("SIZE -", "textSizeDown");
            this.addLayerScreenButton("COLOUR", "TEXT COLOUR");
        } else if (kind === "mask" || kind === "standaloneMask") {
            if (kind === "standaloneMask") {
                let sourceMinus = document.createElement("button");
                sourceMinus.innerText = "SOURCE -";
                sourceMinus.onclick = () => {
                    window.dispatchEvent(new CustomEvent("maskVideoSourceStep", {
                        detail: {
                            direction: -1
                        }
                    }));
                    this.render();
                };
                this.subMenu.appendChild(sourceMinus);
                let sourceDisplay = document.createElement("span");
                sourceDisplay.innerText = "SOURCE: " + (this.maskSelection.sourceLabel || "NONE");
                sourceDisplay.className = "ring-id-display";
                this.subMenu.appendChild(sourceDisplay);
                let sourcePlus = document.createElement("button");
                sourcePlus.innerText = "SOURCE +";
                sourcePlus.onclick = () => {
                    window.dispatchEvent(new CustomEvent("maskVideoSourceStep", {
                        detail: {
                            direction: 1
                        }
                    }));
                    this.render();
                };
                this.subMenu.appendChild(sourcePlus);
            }
            this.addLayerButton("SOLID/VIDEO", "toggleLayerFill");
            this.addLayerScreenButton("COLOUR", "COLOUR");
            this.addLayerButton("THRESHOLD +", "thresholdUp");
            this.addLayerButton("THRESHOLD -", "thresholdDown");
            const matteMode = this.maskSelection.mode || "difference";
            let matteModeButton = document.createElement("button");
            matteModeButton.innerText = "MODE: " + (matteMode === "difference" ? "DIFFERENCE" : "CHROMA");
            matteModeButton.onclick = () => {
                window.dispatchEvent(new Event("toggleMatteMode"));
                this.render();
            };
            this.subMenu.appendChild(matteModeButton);
            /*
            Both context-dependent on the mode - CAPTURE BACKGROUND
            only means anything in difference mode (Segmentation.
            process() keys off capturedBackground there), KEY COLOUR
            only in keying mode - kept after the MODE button (the
            only thing that changes when it's clicked) instead of
            before it, so toggling mode never shifts the MODE button's
            own position or anything before it.
            */
            if (matteMode === "difference") {
                this.addLayerButton("CAPTURE BACKGROUND", "captureLayerBackground");
            } else {
                this.addLayerScreenButton("KEY COLOUR", "KEY COLOUR");
            }
        }
    }
    addLayerButton(label, eventName) {
        let button = document.createElement("button");
        button.innerText = label;
        button.onclick = () => {
            console.log("MENU SELECTED:", label);
            window.dispatchEvent(new Event(eventName));
        };
        this.subMenu.appendChild(button);
    }
    addLayerScreenButton(label, screenKey) {
        let button = document.createElement("button");
        button.innerText = label;
        button.onclick = () => {
            this.enter(screenKey);
        };
        this.subMenu.appendChild(button);
    }
    /*
    This picks one of this RINGS instance's own individual concentric
    ring strokes - a much more granular thing than the RINGS 1/2/3
    node-level numbering shown elsewhere, hence "STROKE" here rather
    than "RING", so the two can't be misread as the same count (one
    RINGS node here still means exactly one node, however many strokes
    it draws). Bounded by this instance's own RING COUNT, never a fixed
    range - only ever offer strokes that actually exist, see CLAUDE.md.
    */
    renderRingColourPicker() {
        const ringCount = this.videoSelection.ringCount || 1;
        if (this.ringId > ringCount) this.ringId = ringCount;
        let minus = document.createElement("button");
        minus.innerText = "STROKE -";
        minus.onclick = () => {
            if (this.ringId > 1) this.ringId--;
            this.render();
        };
        this.subMenu.appendChild(minus);
        let display = document.createElement("span");
        display.innerText = "STROKE " + this.ringId;
        display.className = "ring-id-display";
        this.subMenu.appendChild(display);
        let plus = document.createElement("button");
        plus.innerText = "STROKE +";
        plus.onclick = () => {
            if (this.ringId < ringCount) this.ringId++;
            this.render();
        };
        this.subMenu.appendChild(plus);
        SWATCHES.forEach(swatch => {
            let button = document.createElement("button");
            button.innerText = swatch.label;
            this.styleSwatch(button, swatch.r, swatch.g, swatch.b);
            button.onclick = () => {
                console.log("MENU SELECTED:", "STROKE", this.ringId, swatch.label);
                window.dispatchEvent(new CustomEvent("ringColour", {
                    detail: {
                        ringId: this.ringId,
                        r: swatch.r,
                        g: swatch.g,
                        b: swatch.b
                    }
                }));
            };
            this.subMenu.appendChild(button);
        });
    }
    renderKeyColourPicker() {
        let layerLabel = document.createElement("span");
        layerLabel.innerText = this.maskSelection.label;
        layerLabel.className = "ring-id-display";
        this.subMenu.appendChild(layerLabel);
        let current = document.createElement("span");
        current.innerText = "CURRENT";
        current.className = "ring-id-display";
        this.styleSwatch(current, this.keyColour.r, this.keyColour.g, this.keyColour.b);
        this.subMenu.appendChild(current);
        let pick = document.createElement("button");
        pick.innerText = "PICK FROM VIDEO";
        pick.onclick = () => {
            window.dispatchEvent(new Event("armKeyColourPicker"));
        };
        this.subMenu.appendChild(pick);
        SWATCHES.forEach(swatch => {
            let button = document.createElement("button");
            button.innerText = swatch.label;
            this.styleSwatch(button, swatch.r, swatch.g, swatch.b);
            button.onclick = () => {
                window.dispatchEvent(new CustomEvent("bodyKeyColour", {
                    detail: {
                        r: swatch.r,
                        g: swatch.g,
                        b: swatch.b
                    }
                }));
            };
            this.subMenu.appendChild(button);
        });
    }
    renderMaskPicker() {
        const scope = this.node().scope;
        const state = this.maskState[scope];
        const selection = scope === "mask" ? this.maskSelection : this.videoSelection;
        let layerLabel = document.createElement("span");
        layerLabel.innerText = selection.label;
        layerLabel.className = "ring-id-display";
        this.subMenu.appendChild(layerLabel);
        let sourceMinus = document.createElement("button");
        sourceMinus.innerText = "SOURCE -";
        sourceMinus.onclick = () => {
            window.dispatchEvent(new CustomEvent("maskSourceStep", {
                detail: {
                    direction: -1,
                    scope: scope
                }
            }));
        };
        this.subMenu.appendChild(sourceMinus);
        let sourceDisplay = document.createElement("span");
        sourceDisplay.innerText = state.sourceLabel;
        sourceDisplay.className = "ring-id-display";
        this.subMenu.appendChild(sourceDisplay);
        let sourcePlus = document.createElement("button");
        sourcePlus.innerText = "SOURCE +";
        sourcePlus.onclick = () => {
            window.dispatchEvent(new CustomEvent("maskSourceStep", {
                detail: {
                    direction: 1,
                    scope: scope
                }
            }));
        };
        this.subMenu.appendChild(sourcePlus);
        let channelMinus = document.createElement("button");
        channelMinus.innerText = "CHANNEL -";
        channelMinus.onclick = () => {
            window.dispatchEvent(new CustomEvent("maskChannelStep", {
                detail: {
                    direction: -1,
                    scope: scope
                }
            }));
        };
        this.subMenu.appendChild(channelMinus);
        let channelDisplay = document.createElement("span");
        channelDisplay.innerText = state.channel.toUpperCase();
        channelDisplay.className = "ring-id-display";
        this.subMenu.appendChild(channelDisplay);
        let channelPlus = document.createElement("button");
        channelPlus.innerText = "CHANNEL +";
        channelPlus.onclick = () => {
            window.dispatchEvent(new CustomEvent("maskChannelStep", {
                detail: {
                    direction: 1,
                    scope: scope
                }
            }));
        };
        this.subMenu.appendChild(channelPlus);
    }
    /*
    The only place two nodes get drawn together - see CLAUDE.md. Not a
    layerEditor: a COMPOSITE has no deeper settings behind an EDIT
    bridge, so this one screen is both its own stepper (like every
    other multi-instance kind) and its foreground/background/blend
    controls. Empty state matches every other "nothing created yet"
    screen (NODES/KEY, see renderLayerEditor).
    */
    renderComposePicker() {
        const selection = this.compositeSelection;
        if (!selection.label) {
            let empty = document.createElement("span");
            empty.innerText = "NO COMPOSITES YET";
            empty.className = "ring-id-display";
            this.subMenu.appendChild(empty);
            this.addLayerButton("ADD", "addCompositeLayer");
            return;
        }
        let minus = document.createElement("button");
        minus.innerText = "NODE -";
        minus.onclick = () => {
            window.dispatchEvent(new CustomEvent("compositeIndexStep", {
                detail: {
                    direction: -1
                }
            }));
        };
        this.subMenu.appendChild(minus);
        let display = document.createElement("span");
        display.innerText = selection.label;
        display.className = "ring-id-display";
        this.subMenu.appendChild(display);
        let plus = document.createElement("button");
        plus.innerText = "NODE +";
        plus.onclick = () => {
            window.dispatchEvent(new CustomEvent("compositeIndexStep", {
                detail: {
                    direction: 1
                }
            }));
        };
        this.subMenu.appendChild(plus);
        let fgMinus = document.createElement("button");
        fgMinus.innerText = "FOREGROUND -";
        fgMinus.onclick = () => {
            window.dispatchEvent(new CustomEvent("compositeForegroundStep", {
                detail: {
                    direction: -1
                }
            }));
        };
        this.subMenu.appendChild(fgMinus);
        let fgDisplay = document.createElement("span");
        fgDisplay.innerText = selection.foregroundLabel || "NONE";
        fgDisplay.className = "ring-id-display";
        this.subMenu.appendChild(fgDisplay);
        let fgPlus = document.createElement("button");
        fgPlus.innerText = "FOREGROUND +";
        fgPlus.onclick = () => {
            window.dispatchEvent(new CustomEvent("compositeForegroundStep", {
                detail: {
                    direction: 1
                }
            }));
        };
        this.subMenu.appendChild(fgPlus);
        let bgMinus = document.createElement("button");
        bgMinus.innerText = "BACKGROUND -";
        bgMinus.onclick = () => {
            window.dispatchEvent(new CustomEvent("compositeBackgroundStep", {
                detail: {
                    direction: -1
                }
            }));
        };
        this.subMenu.appendChild(bgMinus);
        let bgDisplay = document.createElement("span");
        bgDisplay.innerText = selection.backgroundLabel || "NONE";
        bgDisplay.className = "ring-id-display";
        this.subMenu.appendChild(bgDisplay);
        let bgPlus = document.createElement("button");
        bgPlus.innerText = "BACKGROUND +";
        bgPlus.onclick = () => {
            window.dispatchEvent(new CustomEvent("compositeBackgroundStep", {
                detail: {
                    direction: 1
                }
            }));
        };
        this.subMenu.appendChild(bgPlus);
        let blendMinus = document.createElement("button");
        blendMinus.innerText = "BLEND -";
        blendMinus.onclick = () => {
            window.dispatchEvent(new CustomEvent("compositeBlendModeStep", {
                detail: {
                    direction: -1
                }
            }));
        };
        this.subMenu.appendChild(blendMinus);
        let blendDisplay = document.createElement("span");
        blendDisplay.innerText = (selection.blendMode || "normal").toUpperCase();
        blendDisplay.className = "ring-id-display";
        this.subMenu.appendChild(blendDisplay);
        let blendPlus = document.createElement("button");
        blendPlus.innerText = "BLEND +";
        blendPlus.onclick = () => {
            window.dispatchEvent(new CustomEvent("compositeBlendModeStep", {
                detail: {
                    direction: 1
                }
            }));
        };
        this.subMenu.appendChild(blendPlus);
        // Stays available even with composites already existing -
        // same as ADD MASK on KEY - otherwise there'd be no way to
        // create a second one.
        this.addLayerButton("ADD", "addCompositeLayer");
    }
    /*
    Ghost's own effect input - which node's appearance feeds the
    trail history. A direct selector, not a stepper: one button per
    other real node that exists (any kind - video, mask, rings, text,
    another ghost), no hidden precondition like a required visibility
    mode first. Different from MASKED BY, which masks Ghost's composited output
    and lives under VIDEO instead.

    Fetches fresh data every time it renders, synchronously, via a
    request object app.js mutates in place rather than a broadcast
    event - dispatching a broadcast from here and having its listener
    call render() would re-enter this same method from inside itself.
    */
    renderApplyToMaskPicker() {
        const request = {
            label: null,
            options: null
        };
        window.dispatchEvent(new CustomEvent("requestApplyToMaskRefresh", {
            detail: request
        }));
        if (request.options) {
            this.applyToMaskState = {
                label: request.label,
                options: request.options
            };
        }
        let current = document.createElement("span");
        current.innerText = "CURRENT: " + this.applyToMaskState.label;
        current.className = "ring-id-display";
        this.subMenu.appendChild(current);
        this.applyToMaskState.options.forEach(option => {
            let button = document.createElement("button");
            button.innerText = option.label;
            button.onclick = () => {
                window.dispatchEvent(new CustomEvent("setApplyToMask", {
                    detail: {
                        id: option.id
                    }
                }));
                this.render();
            };
            this.subMenu.appendChild(button);
        });
    }
    select(item) {
        console.log("MENU SELECTED:", item);
        if (item === "CAMERA ON/OFF") window.dispatchEvent(new Event("toggleCamera"));
        if (item === "ADD RINGS") window.dispatchEvent(new Event("addRingsLayer"));
        if (item === "ADD GHOST") window.dispatchEvent(new Event("addGhostLayer"));
        if (item === "ADD TEXT") window.dispatchEvent(new Event("addTextLayer"));
        if (item === "RECORD ON/OFF") window.dispatchEvent(new Event("toggleRecord"));
        if (item === "ON/OFF") window.dispatchEvent(new Event("toggleRingsEnabled"));
        if (item === "CONSTELLATION ON/OFF") window.dispatchEvent(new Event("toggleConstellation"));
        if (item === "DISTANCE +") window.dispatchEvent(new Event("constellationDistanceUp"));
        if (item === "DISTANCE -") window.dispatchEvent(new Event("constellationDistanceDown"));
        if (item === "OUTPUT SIZE +") window.dispatchEvent(new Event("outputSizeUp"));
        if (item === "OUTPUT SIZE -") window.dispatchEvent(new Event("outputSizeDown"));
        if (item === "PLAY/STOP") window.dispatchEvent(new Event("transportPlayStop"));
        if (item === "MINUTE +") window.dispatchEvent(new Event("transportMinuteUp"));
        if (item === "MINUTE -") window.dispatchEvent(new Event("transportMinuteDown"));
        if (item === "SECOND +") window.dispatchEvent(new Event("transportSecondUp"));
        if (item === "SECOND -") window.dispatchEvent(new Event("transportSecondDown"));
        if (item === "FRAME +") window.dispatchEvent(new Event("transportFrameUp"));
        if (item === "FRAME -") window.dispatchEvent(new Event("transportFrameDown"));
        if (item === "SYNC MINUTE +") window.dispatchEvent(new Event("audioSyncMinuteUp"));
        if (item === "SYNC MINUTE -") window.dispatchEvent(new Event("audioSyncMinuteDown"));
        if (item === "SYNC SECOND +") window.dispatchEvent(new Event("audioSyncSecondUp"));
        if (item === "SYNC SECOND -") window.dispatchEvent(new Event("audioSyncSecondDown"));
        if (item === "SYNC FRAME +") window.dispatchEvent(new Event("audioSyncFrameUp"));
        if (item === "SYNC FRAME -") window.dispatchEvent(new Event("audioSyncFrameDown"));
        if (item === "GHOST +") window.dispatchEvent(new Event("ghostUp"));
        if (item === "GHOST -") window.dispatchEvent(new Event("ghostDown"));
        if (item === "GHOST DELAY +") window.dispatchEvent(new Event("ghostDelayUp"));
        if (item === "GHOST DELAY -") window.dispatchEvent(new Event("ghostDelayDown"));
        if (item === "TEXT SIZE +") window.dispatchEvent(new Event("textSizeUp"));
        if (item === "TEXT SIZE -") window.dispatchEvent(new Event("textSizeDown"));
    }
}
