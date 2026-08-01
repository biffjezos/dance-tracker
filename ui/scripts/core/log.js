/*
==================================================
LOGGING
==================================================
There's no bundler/build-time env split here (see ui/package.json - this
is a static page, `npm run build` only compiles Handlebars templates) so
dev-vs-production can't be a build-time constant. Verbosity is decided at
runtime instead, in priority order:

1. `?logLevel=debug` (or any level name) in the page URL - explicit,
   always wins.
2. `?debug` present at all - shorthand for `?logLevel=debug`.
3. Otherwise: "debug" on localhost/127.0.0.1/file:// (i.e. running the
   source directly), "warn" everywhere else (a deployed build) - so a
   deployed page doesn't spam routine per-click/per-render logs into
   users' consoles, but errors/warnings that matter still show up.

Call setLogLevel() at runtime (e.g. from the browser console) to change
verbosity without editing code or reloading with a different URL.
*/

const LEVELS = { error: 0, warn: 1, info: 2, debug: 3 };

function detectDefaultLevel() {
    try {
        const params = new URLSearchParams(window.location.search);
        const explicit = params.get("logLevel");
        if (explicit && LEVELS[explicit] !== undefined) return explicit;
        if (params.has("debug")) return "debug";
    } catch {
        // location/URLSearchParams unavailable (non-browser context) - fall through.
    }

    const host = window.location?.hostname;
    const isLocal = host === "" || host === "localhost" || host === "127.0.0.1";
    return isLocal ? "debug" : "warn";
}

let currentLevel = detectDefaultLevel();

/** Change verbosity at runtime - e.g. from the browser console: `setLogLevel("debug")`. */
export function setLogLevel(level) {
    if (LEVELS[level] === undefined) return;
    currentLevel = level;
}

export function getLogLevel() {
    return currentLevel;
}

function emit(level, args) {
    if (LEVELS[level] > LEVELS[currentLevel]) return;
    const method = level === "debug" ? "log" : level;
    console[method](...args);
}

/**
 * Leveled logger. `error`/`warn` are for things a user or developer should
 * actually see (a real failure, a fallback being used) - they print by
 * default even in a deployed build. `debug`/`info` are routine, high-
 * frequency trace output (menu renders, per-click state) - they're silent
 * by default and only show up once verbosity is turned up.
 */
export const logger = {
    error: (...args) => emit("error", args),
    warn: (...args) => emit("warn", args),
    info: (...args) => emit("info", args),
    debug: (...args) => emit("debug", args),
};
