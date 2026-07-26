/* tslint:disable */
/* eslint-disable */

export class App {
    free(): void;
    [Symbol.dispose](): void;
    add_apply_mask(content: number, mask: number, channel: string): number;
    add_chroma(source: number, key_r: number, key_g: number, key_b: number, threshold: number, fill_video: boolean, fill_r: number, fill_g: number, fill_b: number): number;
    add_compose(foreground: number, background: number, mode: string): number;
    add_difference(source: number, threshold: number, fill_video: boolean, fill_r: number, fill_g: number, fill_b: number): number;
    add_ghost(source: number, count: number, alpha: number, delay_ticks: number): number;
    add_rings(width: number, height: number, count: number, rings_per_group: number, spacing: number, size: number, stroke_width: number): number;
    add_text(width: number, height: number, content: string, colour: string, size: number): number;
    add_video_source(video: HTMLVideoElement): number;
    capture_background(difference_node: number): void;
    forward(video: HTMLVideoElement, seconds: number): void;
    constructor(width: number, height: number);
    play(video: HTMLVideoElement): void;
    preview_tick(node: number, canvas: HTMLCanvasElement): void;
    render_tick(output_node: number, canvas: HTMLCanvasElement): void;
    rewind(video: HTMLVideoElement, seconds: number): void;
    set_text_content(node_id: number, content: string): void;
    stop(video: HTMLVideoElement): void;
}

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly __wbg_app_free: (a: number, b: number) => void;
    readonly app_add_apply_mask: (a: number, b: number, c: number, d: number, e: number) => number;
    readonly app_add_chroma: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number, i: number, j: number) => number;
    readonly app_add_compose: (a: number, b: number, c: number, d: number, e: number) => number;
    readonly app_add_difference: (a: number, b: number, c: number, d: number, e: number, f: number, g: number) => number;
    readonly app_add_ghost: (a: number, b: number, c: number, d: number, e: number) => number;
    readonly app_add_rings: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number) => [number, number, number];
    readonly app_add_text: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number) => [number, number, number];
    readonly app_add_video_source: (a: number, b: any) => [number, number, number];
    readonly app_capture_background: (a: number, b: number) => [number, number];
    readonly app_forward: (a: number, b: any, c: number) => [number, number];
    readonly app_new: (a: number, b: number) => number;
    readonly app_play: (a: number, b: any) => [number, number];
    readonly app_preview_tick: (a: number, b: number, c: any) => [number, number];
    readonly app_render_tick: (a: number, b: number, c: any) => [number, number];
    readonly app_rewind: (a: number, b: any, c: number) => [number, number];
    readonly app_set_text_content: (a: number, b: number, c: number, d: number) => void;
    readonly app_stop: (a: number, b: any) => [number, number];
    readonly __wbindgen_exn_store: (a: number) => void;
    readonly __externref_table_alloc: () => number;
    readonly __wbindgen_externrefs: WebAssembly.Table;
    readonly __wbindgen_malloc: (a: number, b: number) => number;
    readonly __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
    readonly __externref_table_dealloc: (a: number) => void;
    readonly __wbindgen_start: () => void;
}

export type SyncInitInput = BufferSource | WebAssembly.Module;

/**
 * Instantiates the given `module`, which can either be bytes or
 * a precompiled `WebAssembly.Module`.
 *
 * @param {{ module: SyncInitInput }} module - Passing `SyncInitInput` directly is deprecated.
 *
 * @returns {InitOutput}
 */
export function initSync(module: { module: SyncInitInput } | SyncInitInput): InitOutput;

/**
 * If `module_or_path` is {RequestInfo} or {URL}, makes a request and
 * for everything else, calls `WebAssembly.instantiate` directly.
 *
 * @param {{ module_or_path: InitInput | Promise<InitInput> }} module_or_path - Passing `InitInput` directly is deprecated.
 *
 * @returns {Promise<InitOutput>}
 */
export default function __wbg_init (module_or_path?: { module_or_path: InitInput | Promise<InitInput> } | InitInput | Promise<InitInput>): Promise<InitOutput>;
