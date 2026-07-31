// build-html.js
//
// Compiles templates/index.hbs (plus its partials) into a static index.html.
// This is the entire build step - the shipped page is still plain static
// HTML, just generated instead of hand-duplicated. Run via `npm run build`.

const fs = require("fs");
const path = require("path");
const Handlebars = require("handlebars");

const ROOT = __dirname;
const TEMPLATES_DIR = path.join(ROOT, "templates");
const PARTIALS_DIR = path.join(TEMPLATES_DIR, "partials");
const OUTPUT_FILE = path.join(ROOT, "index.html");

function registerPartials() {
    for (const file of fs.readdirSync(PARTIALS_DIR)) {
        if (!file.endsWith(".hbs")) continue;
        const name = path.basename(file, ".hbs");
        const source = fs.readFileSync(path.join(PARTIALS_DIR, file), "utf8");
        Handlebars.registerPartial(name, source);
    }
}

const data = {
    menus: [
        { key: "project", label: "PROJECT" },
        { key: "input", label: "INPUT" },
        { key: "nodes", label: "NODES" },
        { key: "key", label: "KEY" },
        { key: "generate", label: "GENERATE" },
        { key: "animate", label: "ANIMATE" },
        { key: "compose", label: "COMPOSE" },
        { key: "transform", label: "TRANSFORM" },
        { key: "output", label: "OUTPUT" },
    ],
    panels: {
        preview: {
            sectionClass: "camera-panel",
            titleId: "camera-panel-title",
            titleText: "PREVIEW: NONE",
            panelKey: "preview",
            hasVideo: true,
            canvasId: "camera-preview",
        },
        output: {
            sectionClass: "output-panel",
            titleId: "live-output-title",
            titleText: "LIVE: NONE",
            panelKey: "output",
            screenExtraClass: "output-stack",
            canvasId: "master-layer",
        },
    },
};

function build() {
    registerPartials();
    const templateSource = fs.readFileSync(path.join(TEMPLATES_DIR, "index.hbs"), "utf8");
    const template = Handlebars.compile(templateSource);
    const html = template(data);
    fs.writeFileSync(OUTPUT_FILE, html);
    console.log(`Wrote ${path.relative(ROOT, OUTPUT_FILE)}`);
}

build();
