/* tslint:disable */
/* eslint-disable */

/**
 * 提供给 JS 调用的初始化函数
 */
export function init_mario_game(canvas_id: string): Promise<any>;

/**
 * WASM 启动入口 - 由 JavaScript 显式调用
 */
export function run_game(): void;

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly init_mario_game: (a: number, b: number) => number;
    readonly run_game: () => void;
    readonly __wasm_bindgen_func_elem_287: (a: number, b: number) => void;
    readonly __wasm_bindgen_func_elem_1642: (a: number, b: number) => void;
    readonly __wasm_bindgen_func_elem_2517: (a: number, b: number) => void;
    readonly __wasm_bindgen_func_elem_3615: (a: number, b: number, c: number, d: number) => void;
    readonly __wasm_bindgen_func_elem_348: (a: number, b: number, c: number) => void;
    readonly __wasm_bindgen_func_elem_1688: (a: number, b: number, c: number) => void;
    readonly __wasm_bindgen_func_elem_2533: (a: number, b: number, c: number) => void;
    readonly __wasm_bindgen_func_elem_347: (a: number, b: number) => void;
    readonly __wbindgen_export: (a: number, b: number) => number;
    readonly __wbindgen_export2: (a: number, b: number, c: number, d: number) => number;
    readonly __wbindgen_export3: (a: number) => void;
    readonly __wbindgen_export4: (a: number, b: number, c: number) => void;
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
