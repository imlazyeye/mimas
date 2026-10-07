// The sprite editor: the whole 128 by 128 sheet at 2x for choosing a sprite, and the chosen 8 by 8
// sprite at 24x for drawing on. `sprites.txt` is the only copy of the sheet: edits from elsewhere
// are read back in, and an edit here writes the file out again through `store.setFile`.

import { button, h } from './dom.js';
import { PALETTE } from './palette.js';

const FILE = 'sprites.txt';
const SIZE = 128;
const CELL = 8;
const PER_ROW = SIZE / CELL;
const SPRITES = PER_ROW * PER_ROW;
const SHEET_SCALE = 2;
const EDIT_SCALE = 24;
const UNDO_LIMIT = 50;
const HEX = '0123456789abcdef';

const GRID = 'rgba(244, 244, 244, 0.12)';
const SELECTED = '#ffcd75';

const RGB = PALETTE.map(([, hex]) => [1, 3, 5].map((at) => parseInt(hex.slice(at, at + 2), 16)));

// the sheet and its file. a sprite is a Uint8Array of 64 palette indices, row by row

function parse(text, sheet) {
    const lines = text.split('\n');
    if (lines.at(-1) === '') lines.pop();
    let clean = true;
    sheet.fill(0);
    lines.forEach((raw, y) => {
        const line = raw.endsWith('\r') ? raw.slice(0, -1) : raw;
        if (y >= SIZE) {
            clean = clean && line === '';
            return;
        }
        for (let x = 0; x < line.length; x++) {
            const digit = HEX.indexOf(line[x].toLowerCase());
            if (x >= SIZE || digit < 0) clean = false;
            else sheet[y * SIZE + x] = digit;
        }
    });
    return clean;
}

// the lines the sheet needs: a row ends at its last lit pixel, and the rows after the last lit
// one are left out (the console reads what's missing as black)
function serialize(sheet) {
    const rows = [];
    for (let y = 0; y < SIZE; y++) {
        let row = '';
        for (let x = 0; x < SIZE; x++) row += HEX[sheet[y * SIZE + x]];
        rows.push(row.replace(/0+$/, ''));
    }
    while (rows.length > 0 && rows.at(-1) === '') rows.pop();
    return rows.map((row) => `${row}\n`).join('');
}

function origin(n) {
    return Math.floor(n / PER_ROW) * CELL * SIZE + (n % PER_ROW) * CELL;
}

function grab(sheet, n) {
    const pixels = new Uint8Array(CELL * CELL);
    const at = origin(n);
    for (let i = 0; i < pixels.length; i++) pixels[i] = sheet[at + (i >> 3) * SIZE + (i & 7)];
    return pixels;
}

function put(sheet, n, pixels) {
    const at = origin(n);
    for (let i = 0; i < pixels.length; i++) sheet[at + (i >> 3) * SIZE + (i & 7)] = pixels[i];
}

// a stroke plots a line from where the pointer was to where it is, so a fast drag leaves no gaps
function line(x0, y0, x1, y1, plot) {
    const dx = Math.abs(x1 - x0);
    const dy = -Math.abs(y1 - y0);
    const sx = x0 < x1 ? 1 : -1;
    const sy = y0 < y1 ? 1 : -1;
    let error = dx + dy;
    for (;;) {
        plot(x0, y0);
        if (x0 === x1 && y0 === y1) return;
        const doubled = 2 * error;
        if (doubled >= dy) {
            error += dy;
            x0 += sx;
        }
        if (doubled <= dx) {
            error += dx;
            y0 += sy;
        }
    }
}

// the cell a pointer is over in a canvas of `cells` by `cells`, clamped when a drag leaves it
function cellAt(canvas, event, cells) {
    const rect = canvas.getBoundingClientRect();
    const pick = (offset, length) =>
        Math.min(Math.max(Math.floor((offset / length) * cells), 0), cells - 1);
    return [
        pick(event.clientX - rect.left, rect.width),
        pick(event.clientY - rect.top, rect.height),
    ];
}

export function mountPixels(el, store) {
    const sheet = new Uint8Array(SIZE * SIZE);
    const undo = [];
    const redo = [];
    let sprite = 0;
    let color = 12;
    let stroke = null;
    let writing = false;
    let clean = true;

    // the sheet is drawn into a canvas of its own size first, then scaled into the two views
    const stage = h('canvas', { width: SIZE, height: SIZE });
    const stageContext = stage.getContext('2d');
    const image = stageContext.createImageData(SIZE, SIZE);

    const sheetSize = SIZE * SHEET_SCALE;
    const sheetCanvas = h('canvas', {
        class: 'px-sheet',
        width: sheetSize,
        height: sheetSize,
        tabindex: '0',
        'aria-label': 'Sprite sheet, the arrow keys choose a sprite',
    });
    const sheetContext = sheetCanvas.getContext('2d');
    sheetContext.imageSmoothingEnabled = false;

    const spriteSize = CELL * EDIT_SCALE;
    const spriteCanvas = h('canvas', {
        class: 'px-sprite',
        width: spriteSize,
        height: spriteSize,
        role: 'img',
        'aria-label': 'The chosen sprite, drawn on with the mouse or touch',
    });
    const spriteContext = spriteCanvas.getContext('2d');
    spriteContext.imageSmoothingEnabled = false;

    const caption = h('p', { class: 'px-caption', 'aria-live': 'polite' });
    const chip = h('span', { class: 'px-chip' });
    const colorName = h('span');
    const notice = h(
        'p',
        { class: 'px-notice', role: 'status' },
        "sprites.txt has text the sheet can't show. Editing here rewrites it without that text.",
    );

    const swatches = PALETTE.map(([name, hex], i) => {
        const swatch = button(
            '',
            () => {
                color = i;
                sync();
            },
            { class: 'px-swatch', title: `${name} ${hex}, digit ${HEX[i]}`, 'aria-label': name },
        );
        swatch.style.background = hex;
        return swatch;
    });

    const undoButton = button('Undo', () => travel(undo, redo), { title: 'Ctrl+Z' });
    const redoButton = button('Redo', () => travel(redo, undo), { title: 'Ctrl+Shift+Z' });

    const root = h(
        'div',
        { class: 'px', tabindex: '-1' },
        notice,
        h('div', { class: 'px-col' }, sheetCanvas, caption),
        h(
            'div',
            { class: 'px-col' },
            spriteCanvas,
            h('div', { class: 'px-palette', role: 'group', 'aria-label': 'Palette' }, ...swatches),
            h('p', { class: 'px-current' }, chip, colorName),
            h('div', { class: 'px-row', role: 'group', 'aria-label': 'History' }, undoButton, redoButton),
            h(
                'p',
                { class: 'px-hint' },
                'The left button draws and the right one erases to Black. Ctrl+Z and Ctrl+Shift+Z undo and redo.',
            ),
        ),
    );
    el.replaceChildren(root);

    // drawing

    function paint() {
        const data = image.data;
        for (let i = 0; i < sheet.length; i++) {
            const [r, g, b] = RGB[sheet[i]];
            data[i * 4] = r;
            data[i * 4 + 1] = g;
            data[i * 4 + 2] = b;
            data[i * 4 + 3] = 255;
        }
        stageContext.putImageData(image, 0, 0);

        sheetContext.drawImage(stage, 0, 0, sheetSize, sheetSize);
        sheetContext.fillStyle = GRID;
        for (let i = 1; i < PER_ROW; i++) {
            const at = i * CELL * SHEET_SCALE;
            sheetContext.fillRect(at, 0, 1, sheetSize);
            sheetContext.fillRect(0, at, sheetSize, 1);
        }
        const cell = CELL * SHEET_SCALE;
        sheetContext.strokeStyle = SELECTED;
        sheetContext.lineWidth = 2;
        sheetContext.strokeRect(
            (sprite % PER_ROW) * cell + 1,
            Math.floor(sprite / PER_ROW) * cell + 1,
            cell - 2,
            cell - 2,
        );

        const sx = (sprite % PER_ROW) * CELL;
        const sy = Math.floor(sprite / PER_ROW) * CELL;
        spriteContext.drawImage(stage, sx, sy, CELL, CELL, 0, 0, spriteSize, spriteSize);
        spriteContext.fillStyle = GRID;
        for (let i = 1; i < CELL; i++) {
            spriteContext.fillRect(i * EDIT_SCALE, 0, 1, spriteSize);
            spriteContext.fillRect(0, i * EDIT_SCALE, spriteSize, 1);
        }
    }

    function sync() {
        swatches.forEach((node, i) => node.setAttribute('aria-pressed', String(i === color)));
        const [name, hex] = PALETTE[color];
        chip.style.background = hex;
        colorName.textContent = `${name} ${hex}, digit ${HEX[color]}`;
        const x = (sprite % PER_ROW) * CELL;
        const y = Math.floor(sprite / PER_ROW) * CELL;
        const text = `Sprite ${sprite} (x ${x}, y ${y})`;
        if (caption.textContent !== text) caption.textContent = text;
        undoButton.disabled = undo.length === 0;
        redoButton.disabled = redo.length === 0;
        notice.hidden = clean;
    }

    function choose(n) {
        sprite = Math.min(Math.max(n, 0), SPRITES - 1);
        paint();
        sync();
    }

    // changing the sheet, where a stroke is one undo step

    function save() {
        writing = true;
        try {
            store.setFile(FILE, serialize(sheet));
            clean = true;
        } finally {
            writing = false;
        }
    }

    function travel(from, to) {
        const step = from.pop();
        if (!step) return;
        to.push({ n: step.n, pixels: grab(sheet, step.n) });
        sprite = step.n;
        put(sheet, step.n, step.pixels);
        paint();
        save();
        sync();
    }

    function reload() {
        stroke = null;
        undo.length = 0;
        redo.length = 0;
        clean = parse(store.cart?.files?.[FILE] ?? '', sheet);
        paint();
        sync();
    }

    function draw([x, y]) {
        line(stroke.last[0], stroke.last[1], x, y, (px, py) => {
            stroke.pixels[py * CELL + px] = stroke.value;
        });
        stroke.last = [x, y];
        put(sheet, stroke.n, stroke.pixels);
        paint();
    }

    function finish() {
        if (!stroke) return;
        const { n, before } = stroke;
        stroke = null;
        if (grab(sheet, n).every((value, i) => value === before[i])) return;
        undo.push({ n, pixels: before });
        if (undo.length > UNDO_LIMIT) undo.shift();
        redo.length = 0;
        save();
        sync();
    }

    spriteCanvas.addEventListener('pointerdown', (event) => {
        if (event.button !== 0 && event.button !== 2) return;
        spriteCanvas.setPointerCapture(event.pointerId);
        const start = cellAt(spriteCanvas, event, CELL);
        const before = grab(sheet, sprite);
        stroke = {
            n: sprite,
            before,
            pixels: before.slice(),
            value: event.button === 2 ? 0 : color,
            last: start,
        };
        draw(start);
    });
    spriteCanvas.addEventListener('pointermove', (event) => {
        if (stroke) draw(cellAt(spriteCanvas, event, CELL));
    });
    spriteCanvas.addEventListener('pointerup', finish);
    spriteCanvas.addEventListener('pointercancel', finish);
    spriteCanvas.addEventListener('contextmenu', (event) => event.preventDefault());

    // choosing a sprite

    sheetCanvas.addEventListener('pointerdown', (event) => {
        const [x, y] = cellAt(sheetCanvas, event, PER_ROW);
        choose(y * PER_ROW + x);
    });

    sheetCanvas.addEventListener('keydown', (event) => {
        const moves = {
            ArrowLeft: [-1, 0],
            ArrowRight: [1, 0],
            ArrowUp: [0, -1],
            ArrowDown: [0, 1],
        };
        const move = moves[event.key];
        if (!move) return;
        event.preventDefault();
        const x = Math.min(Math.max((sprite % PER_ROW) + move[0], 0), PER_ROW - 1);
        const y = Math.min(Math.max(Math.floor(sprite / PER_ROW) + move[1], 0), PER_ROW - 1);
        choose(y * PER_ROW + x);
    });

    root.addEventListener('keydown', (event) => {
        if (!(event.ctrlKey || event.metaKey) || event.altKey) return;
        const key = event.key.toLowerCase();
        if (key === 'z') {
            event.preventDefault();
            if (event.shiftKey) travel(redo, undo);
            else travel(undo, redo);
        } else if (key === 'y') {
            event.preventDefault();
            travel(redo, undo);
        }
    });

    store.on('cart', reload);
    store.on('file', (path) => {
        if (path === FILE && !writing) reload();
    });
    reload();
}
