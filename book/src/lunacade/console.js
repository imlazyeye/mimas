// The console on the page: a canvas the machine's screen is copied into, the keyboard and mouse
// as the machine wants them, and a loop that runs the cart's frames at 60 a second.

import { h } from './dom.js';

const WIDTH = 256;
const HEIGHT = 144;
const FRAME = 1000 / 60;

// how many cart frames run for one frame of the page. time past that is dropped, so a slow
// machine slows the game down instead of falling behind for good
const MAX_FRAMES = 2;

// which keys press which button, as the bit `Console.frame` wants: up, down, left, right, A, B,
// X, Y, Start
const KEYS = {
    ArrowUp: 0,
    KeyW: 0,
    ArrowDown: 1,
    KeyS: 1,
    ArrowLeft: 2,
    KeyA: 2,
    ArrowRight: 3,
    KeyD: 3,
    KeyZ: 4,
    KeyJ: 4,
    KeyX: 5,
    KeyK: 5,
    KeyC: 6,
    KeyL: 6,
    KeyV: 7,
    Semicolon: 7,
    Enter: 8,
};

// `hooks` gets `loaded()` when a cart has taken over, before its first output, `print(line)`,
// `fault(diagnostic)`, `stats({fps, ms, state})` twice a second and `crash(error)` when the wasm
// throws, after which nothing runs until the next `load`
export function mountConsole(frame, hooks) {
    const canvas = h('canvas', {
        width: WIDTH,
        height: HEIGHT,
        tabindex: 0,
        'aria-label': 'The console. Click it so it has the keyboard.',
    });
    frame.prepend(canvas);
    const context = canvas.getContext('2d');
    const image = context.createImageData(WIDTH, HEIGHT);
    const rgba = new Uint8Array(image.data.buffer);

    let console = null;
    let paused = false;
    let halted = false;
    let held = 0;
    const keys = new Set();
    let pressed = 0;
    let released = 0;
    let mouse = [-1, -1];
    let mouseHeld = 0;
    let mousePressed = 0;
    let owed = 0;
    let last = 0;
    let ran = 0;
    let ms = 0;
    let tick = 0;

    // the screen keeps a whole number of device pixels for each of its own, in the middle of the
    // frame
    new ResizeObserver(() => {
        const dpr = window.devicePixelRatio || 1;
        const fit = Math.min((frame.clientWidth * dpr) / WIDTH, (frame.clientHeight * dpr) / HEIGHT);
        const scale = Math.max(1, Math.floor(fit)) / dpr;
        canvas.style.width = `${WIDTH * scale}px`;
        canvas.style.height = `${HEIGHT * scale}px`;
    }).observe(frame);

    function key(event, down) {
        const bit = KEYS[event.code];
        if (bit === undefined) return;
        const before = held;
        if (down) keys.add(event.code);
        else keys.delete(event.code);
        held = 0;
        for (const code of keys) held |= 1 << KEYS[code];
        pressed |= held & ~before;
        released |= before & ~held;
        event.preventDefault();
    }
    canvas.addEventListener('keydown', (event) => key(event, true));
    canvas.addEventListener('keyup', (event) => key(event, false));
    canvas.addEventListener('blur', () => {
        keys.clear();
        released |= held;
        held = 0;
        mouseHeld = 0;
    });
    function moveMouse(event) {
        const rect = canvas.getBoundingClientRect();
        const x = Math.floor(((event.clientX - rect.left) / rect.width) * WIDTH);
        const y = Math.floor(((event.clientY - rect.top) / rect.height) * HEIGHT);
        mouse = [x, y];
    }
    canvas.addEventListener('pointermove', moveMouse);
    canvas.addEventListener('pointerleave', () => (mouse = [-1, -1]));
    canvas.addEventListener('pointerdown', (event) => {
        if (event.button !== 0 && event.button !== 2) return;
        moveMouse(event);
        const bit = 1 << (event.button === 2 ? 1 : 0);
        mousePressed |= bit & ~mouseHeld;
        mouseHeld |= bit;
        canvas.focus({ preventScroll: true });
        event.preventDefault();
    });
    window.addEventListener('pointerup', (event) => {
        if (event.button === 0 || event.button === 2) {
            mouseHeld &= ~(1 << (event.button === 2 ? 1 : 0));
        }
    });
    canvas.addEventListener('contextmenu', (event) => event.preventDefault());

    function state() {
        if (!console) return 'idle';
        if (halted) return 'halted';
        return paused ? 'paused' : 'running';
    }

    function show() {
        console.screen(rgba);
        context.putImageData(image, 0, 0);
    }

    function report() {
        for (const line of console.take_output()) hooks.print(line);
        const fault = console.fault();
        if (fault) {
            halted = true;
            hooks.fault(JSON.parse(fault));
        }
    }

    function crash(error) {
        console = null;
        hooks.crash(error);
    }

    function loop(now) {
        requestAnimationFrame(loop);
        const passed = Math.min(now - last, 100);
        last = now;
        if (now - tick >= 500) {
            tick = now;
            hooks.stats({ fps: ran * 2, ms, state: state() });
            ran = 0;
        }
        if (!console || paused || halted) return;
        owed += passed;
        let drew = false;
        for (let i = 0; i < MAX_FRAMES && owed >= FRAME; i += 1) {
            owed -= FRAME;
            const start = performance.now();
            try {
                const drewFrame = console.frame(
                    held, pressed, released, mouse[0], mouse[1], mouseHeld, mousePressed,
                );
                pressed = released = mousePressed = 0;
                if (drewFrame) {
                    ms = performance.now() - start;
                    ran += 1;
                    drew = true;
                }
                report();
            } catch (error) {
                crash(error);
                return;
            }
            if (halted) break;
        }
        owed = Math.min(owed, FRAME);
        if (drew) show();
    }
    requestAnimationFrame(loop);

    return {
        // loads a cart from the wasm module's `Console` class, and gives back the diagnostics of
        // one that doesn't load (the one that was running keeps running then)
        load(Console, files, seed) {
            let next;
            try {
                next = new Console(JSON.stringify({ files }), seed);
            } catch (error) {
                if (typeof error !== 'string') throw error;
                return JSON.parse(error);
            }
            console?.free();
            console = next;
            pressed = released = mousePressed = 0;
            halted = false;
            paused = false;
            owed = 0;
            hooks.loaded();
            try {
                show();
                report();
            } catch (error) {
                crash(error);
            }
            return null;
        },
        stop() {
            console?.free();
            console = null;
        },
        get paused() {
            return paused;
        },
        set paused(value) {
            paused = value;
        },
        get state() {
            return state();
        },
        focus() {
            canvas.focus({ preventScroll: true });
        },
    };
}
