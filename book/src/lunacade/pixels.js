// The pixel editor is a cart. This only connects its console and sprite sheet to the page's
// file store; painting, selection, the palette and history are all in pixels.mim.
import { mountConsole } from './console.js';
import { h } from './dom.js';

const FILE = 'sprites.txt';

export function mountPixels(el, store) {
    let writing = false;
    const notice = h('p', { class: 'px-notice', role: 'status', hidden: true });
    const frame = h('div', { class: 'frame' });
    const hint = h('p', { class: 'px-hint' },
        'Choose a sprite on the left, then draw on the right. Right-click erases to Black. ',
        'Arrow keys choose a sprite. Z or Ctrl+Z undoes; X, Ctrl+Shift+Z or Ctrl+Y redoes. ',
        'On Mac, use Command in place of Ctrl.');
    el.replaceChildren(h('div', { class: 'px' }, frame, notice, hint));

    function failed(message) {
        if (notice.textContent !== message) notice.textContent = message;
        notice.hidden = false;
    }

    const screen = mountConsole(frame, {
        loaded() { notice.hidden = true; },
        print() {},
        stats() {},
        fault(diagnostic) { failed(`The sprite editor stopped: ${diagnostic.message}`); },
        crash(error) {
            console.error(error);
            failed('The sprite editor stopped. Reload the page to start it again.');
        },
        changed(console) {
            const text = console.take_sheet();
            if (text !== undefined) {
                writing = true;
                try { store.setFile(FILE, text); }
                finally { writing = false; }
                notice.hidden = true;
            }
            if (!console.sheet_is_clean()) {
                failed("sprites.txt has text the sheet can't show. Editing here rewrites it without that text.");
            }
        },
    }, {
        label: 'Sprite editor. Choose a sprite, draw, select a color, undo or redo.',
        clampDrag: true,
        shortcut(event) {
            if (!(event.ctrlKey || event.metaKey) || event.altKey) return;
            if (event.code === 'KeyZ') return event.shiftKey ? 5 : 4;
            if (event.code === 'KeyY') return 5;
        },
    });

    function reload() {
        screen.stop();
        if (!store.runtime) return;
        try {
            const problems = screen.load(() => store.runtime.Console.pixel_editor(store.cart.files[FILE] ?? ''));
            if (problems) failed(problems.map((problem) => problem.message).join('\n'));
        } catch (error) {
            console.error(error);
            failed("The sprite editor couldn't start. Reload the page to try again.");
        }
        screen.paused = store.tab !== 'sprites';
    }

    store.on('cart', reload);
    store.on('runtime', reload);
    store.on('file', (path) => { if (path === FILE && !writing) reload(); });
    store.on('tab', (name) => { screen.paused = name !== 'sprites'; });
    reload();
}
