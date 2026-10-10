// The page's state and its hold on the console. app.js owns the open cart, the saving, the checks
// and runs, and the wasm module, and the editor and sprite editor get all of it through the store
// they're mounted with.
//
// The store is `cart` ({id, files}, a new object whenever the cart changes and never null in a
// mount), `diagnostics` (the last check or failed load, plus the fault if there is one),
// `on(event, fn)`, `setFile(path, text)` (`null` deletes, so a rename is a set and a delete),
// `select(path, range)` (shows the Code tab and emits "select") and `focusConsole()`. The events
// are "cart" (read `store.cart`), "file" (the path that changed), "diagnostics" (the list) and
// "select" (`{path, range}`, a range having the shape of a diagnostic), "tab" (the active tab)
// and "runtime" (the wasm module was stopped or replaced). The sprite editor also reads `tab`
// and `runtime` so its own console pauses when hidden and recovers with the game console.

import { mountConsole } from './console.js';
import { decodeCart, encodeCart, validFiles } from './share.js';

const KEY = 'lunacade:v2:cart:';
const CURRENT = 'lunacade:v2:current';
const AUTORUN = 'lunacade:v2:autorun';

const CHECK_DELAY = 300;
const RUN_DELAY = 600;
const SAVE_DELAY = 500;
const LOG_LINES = 300;

const RESET = 'Reset to original';

const $ = (id) => document.getElementById(id);

const frame = $('frame');
const notice = $('notice');
const picker = $('picker');
const restartButton = $('restart');
const pauseButton = $('pause');
const shareButton = $('share');
const resetButton = $('reset');
const autorunBox = $('autorun');
const log = $('log');
const starting = $('starting');

// storage can be blocked or full, and the page works without it

function read(key) {
    try {
        return localStorage.getItem(key);
    } catch {
        return null;
    }
}

function write(key, value) {
    try {
        localStorage.setItem(key, value);
        return true;
    } catch {
        return false;
    }
}

function remove(key) {
    try {
        localStorage.removeItem(key);
    } catch {}
}

// the store

const listeners = new Map();

function on(event, fn) {
    if (!listeners.has(event)) listeners.set(event, new Set());
    listeners.get(event).add(fn);
    return () => listeners.get(event).delete(fn);
}

function emit(event, arg) {
    for (const fn of listeners.get(event) ?? []) {
        try {
            fn(arg);
        } catch (error) {
            console.error(error);
        }
    }
}

const store = {
    cart: null,
    tab: 'code',
    diagnostics: [],
    on,
    setFile,
    select,
    // a `.mim` file that isn't a module is the cart's script
    isModule: (text) => wasm.is_module(text),
    focusConsole: () => screen.focus(),
    // Queries only see analysis of the current files. Rename and Run can check immediately.
    get lsp() {
        return analysisCurrent ? wasm : null;
    },
    ensureAnalysis,
    notify,
    get runtime() { return wasm; },
};

// the carts: the examples from the console, and a cart from a link

const originals = new Map();
let examples = [];
let sharedFrom = null;

function title(id) {
    return id === 'shared' ? 'Shared cart' : id[0].toUpperCase() + id.slice(1);
}

function renderPicker() {
    const ids = [...examples.map((cart) => cart.id).filter((id) => id !== 'new')];
    if (originals.has('shared')) ids.push('shared');
    if (originals.has('new')) ids.push('new');
    picker.replaceChildren(...ids.map((id) => new Option(title(id), id)));
    if (store.cart) picker.value = store.cart.id;
}

function openCart(id) {
    const base = originals.get(id);
    if (!base) return;
    saveNow();
    const files = { ...(savedFiles(id) ?? base.files) };
    store.cart = { id, files };
    analysisCurrent = false;
    checkList = [];
    fault = null;
    notice.hidden = true;
    $('share-row').hidden = true;
    disarmReset();
    setSaved(null);
    if (id !== 'shared') write(CURRENT, id);
    picker.value = id;
    $('cart-name').textContent = title(id);
    publishDiagnostics();
    emit('cart');
    check();
    runNow();
}

function resetCart() {
    const base = originals.get(store.cart.id);
    clearTimeout(saveTimer);
    dirty = false;
    remove(KEY + store.cart.id);
    store.cart.files = { ...base.files };
    analysisCurrent = false;
    checkList = [];
    fault = null;
    notice.hidden = true;
    setSaved(null);
    publishDiagnostics();
    emit('cart');
    check();
    runNow();
}

// setting a file saves it, and checks it and runs it once typing has stopped for a moment

let checkTimer = 0;
let runTimer = 0;
let runPending = false;
let autorun = read(AUTORUN) !== 'off';
let checkList = [];
let fault = null;
let analysisCurrent = false;

function setFile(path, text) {
    if (!store.cart) return;
    const files = store.cart.files;
    if (text === null) {
        if (!Object.hasOwn(files, path)) return;
        delete files[path];
    } else {
        if (files[path] === text) return;
        files[path] = text;
    }
    analysisCurrent = false;
    scheduleSave();
    clearTimeout(checkTimer);
    checkTimer = setTimeout(check, CHECK_DELAY);
    clearTimeout(runTimer);
    runPending = false;
    runTimer = setTimeout(() => {
        runPending = true;
        tryRun();
    }, RUN_DELAY);
    emit('file', path);
}

// a check also builds the analysis the editor asks about, so an open cart gets one right away
function check() {
    clearTimeout(checkTimer);
    checkTimer = 0;
    if (!wasm || !store.cart) return;
    try {
        checkList = JSON.parse(wasm.check_cart(JSON.stringify({ files: store.cart.files })));
        analysisCurrent = true;
    } catch (error) {
        crashed(error);
        return;
    }
    fault = null;
    publishDiagnostics();
    tryRun();
}

function ensureAnalysis() {
    if (!analysisCurrent) check();
    return analysisCurrent;
}

function tryRun() {
    if (!runPending || checkTimer) return;
    runPending = false;
    if (autorun && checkList.length === 0) runNow();
}

function runNow() {
    if (!store.cart || !wasm) return;
    clearTimeout(runTimer);
    runPending = false;
    if (!ensureAnalysis()) return;
    let problems;
    try {
        problems = screen.load(() => new wasm.Console(
            JSON.stringify({ files: store.cart.files }), Math.floor(Math.random() * 2 ** 53),
        ));
    } catch (error) {
        crashed(error);
        return;
    }
    if (problems) {
        checkList = problems;
        publishDiagnostics();
    }
}

function publishDiagnostics() {
    store.diagnostics = fault ? [...checkList, fault] : checkList;
    const count = store.diagnostics.length;
    const badge = $('errors');
    badge.hidden = count === 0;
    badge.textContent = count;
    emit('diagnostics', store.diagnostics);
}

// saving

let saveTimer = 0;
let dirty = false;

function scheduleSave() {
    dirty = true;
    clearTimeout(saveTimer);
    saveTimer = setTimeout(saveNow, SAVE_DELAY);
}

function saveNow() {
    clearTimeout(saveTimer);
    if (!dirty || !store.cart) return;
    dirty = false;
    const { id, files } = store.cart;
    const record = id === 'shared' ? { id, files, from: sharedFrom } : { id, files };
    setSaved(write(KEY + id, JSON.stringify(record)));
}

function savedFiles(id) {
    const text = read(KEY + id);
    if (!text) return null;
    try {
        const saved = JSON.parse(text);
        if (id === 'shared' && saved.from !== sharedFrom) return null;
        return validFiles(saved.files) ? saved.files : null;
    } catch {
        return null;
    }
}

function setSaved(ok) {
    $('saved').textContent = ok === null ? '' : ok ? 'Saved' : 'Not saved (storage is off)';
}

window.addEventListener('pagehide', saveNow);
document.addEventListener('visibilitychange', () => {
    if (document.visibilityState === 'hidden') saveNow();
});

// the console and what it says

let wasm = null;
let generation = 0;
let noticeTimer = 0;
let resetTimer = 0;

const screen = mountConsole(frame, {
    loaded() {
        checkList = [];
        fault = null;
        clearLog();
        setPaused(false);
        publishDiagnostics();
    },
    print: (line) => printLine(line, ''),
    fault(diagnostic) {
        fault = diagnostic;
        publishDiagnostics();
        printLine(describe(diagnostic), 'fault');
    },
    stats: renderStats,
    crash: crashed,
});

// the wasm is loaded again under a new url when it has crashed, since a panic leaves the old
// instance unusable
async function boot() {
    const module = await import(generation ? `./wasm/lunacade.js?reload=${generation}` : './wasm/lunacade.js');
    await module.default();
    return module;
}

function crashed(error) {
    console.error('the console crashed:', error);
    wasm = null;
    analysisCurrent = false;
    screen.stop();
    emit('runtime');
    renderStats({ state: 'crashed' });
    notify('The console crashed. Press Restart to start it again.', true);
}

async function recover() {
    generation += 1;
    try {
        wasm = await boot();
    } catch (error) {
        console.error(error);
        notify("The console couldn't start again. Reload the page to try again.", true);
        return false;
    }
    notice.hidden = true;
    emit('runtime');
    return true;
}

function notify(text, sticky = false) {
    notice.textContent = text;
    notice.hidden = false;
    clearTimeout(noticeTimer);
    if (!sticky) noticeTimer = setTimeout(() => (notice.hidden = true), 6000);
}

// the status bar and the output

function renderStats({ state, fps, ms }) {
    const el = $('state');
    el.dataset.state = state;
    el.textContent = state === 'running' ? `${fps} fps` : state;
    el.title = state === 'running' && Number.isFinite(ms) ? `${ms.toFixed(2)} ms a frame` : '';
}

function describe(diagnostic) {
    const { file, line, col, message } = diagnostic;
    return file ? `${file}:${line}:${col} ${message}` : message;
}

const pending = [];
let flushTimer = 0;

function printLine(text, className) {
    pending.push([text, className]);
    if (pending.length > LOG_LINES) pending.splice(0, pending.length - LOG_LINES);
    if (!flushTimer) flushTimer = setTimeout(flush, 100);
}

function flush() {
    flushTimer = 0;
    const atBottom = log.scrollHeight - log.scrollTop - log.clientHeight < 24;
    for (const [text, className] of pending.splice(0)) {
        const row = document.createElement('div');
        if (className) row.className = className;
        row.textContent = text;
        log.append(row);
    }
    while (log.childElementCount > LOG_LINES) log.firstElementChild.remove();
    if (atBottom) log.scrollTop = log.scrollHeight;
}

function clearLog() {
    pending.length = 0;
    log.replaceChildren();
}

// pausing and the other buttons

function setPaused(value) {
    screen.paused = value;
    pauseButton.textContent = value ? 'Resume' : 'Pause';
}

function disarmReset() {
    clearTimeout(resetTimer);
    resetButton.classList.remove('armed');
    resetButton.textContent = RESET;
}

restartButton.addEventListener('click', async () => {
    if (!wasm && !(await recover())) return;
    runNow();
    screen.focus();
});

pauseButton.addEventListener('click', () => {
    setPaused(!screen.paused);
    screen.focus();
});

resetButton.addEventListener('click', () => {
    if (!store.cart) return;
    if (!resetButton.classList.contains('armed')) {
        resetButton.classList.add('armed');
        resetButton.textContent = 'Click again to reset';
        resetTimer = setTimeout(disarmReset, 3000);
        return;
    }
    disarmReset();
    resetCart();
    screen.focus();
});

resetButton.addEventListener('blur', disarmReset);

$('clear').addEventListener('click', clearLog);
$('run').addEventListener('click', runNow);

autorunBox.checked = autorun;
autorunBox.addEventListener('change', () => {
    autorun = autorunBox.checked;
    write(AUTORUN, autorun ? 'on' : 'off');
});

document.addEventListener('keydown', (event) => {
    if ((event.ctrlKey || event.metaKey) && event.key === 'Enter' && store.cart) {
        event.preventDefault();
        runNow();
    }
});

picker.addEventListener('change', () => {
    if (location.hash.startsWith('#cart=')) {
        history.replaceState(null, '', location.pathname + location.search);
    }
    openCart(picker.value);
    screen.focus();
});

shareButton.addEventListener('click', async () => {
    const note = $('share-note');
    const field = $('share-url');
    $('share-row').hidden = false;
    note.textContent = '';
    try {
        const data = await encodeCart({ id: store.cart.id, files: store.cart.files });
        field.value = `${location.origin}${location.pathname}${location.search}#cart=${data}`;
        const size = `${field.value.length.toLocaleString('en-US')} characters`;
        try {
            await navigator.clipboard.writeText(field.value);
            note.textContent = `Copied (${size}).`;
        } catch {
            field.select();
            note.textContent = `Copy it from the field (${size}).`;
        }
    } catch (error) {
        console.error(error);
        field.value = '';
        note.textContent = "Couldn't make a link.";
    }
});

$('share-url').addEventListener('focus', (event) => event.target.select());

// opening a cart from a link

async function openFragment() {
    const match = /^#cart=([\w=-]+)$/.exec(location.hash);
    if (!match) return false;
    try {
        const cart = await decodeCart(match[1]);
        sharedFrom = match[1];
        originals.set('shared', { id: 'shared', files: { ...cart.files } });
        return true;
    } catch (error) {
        console.error(error);
        notify("That link doesn't hold a cart.");
        return false;
    }
}

window.addEventListener('hashchange', async () => {
    if (!store.cart || !(await openFragment())) return;
    renderPicker();
    openCart('shared');
});

// the tabs

const tabs = [...document.querySelectorAll('[role="tab"]')];

function showTab(name) {
    store.tab = name;
    for (const tab of tabs) {
        const selected = tab.dataset.tab === name;
        tab.setAttribute('aria-selected', selected);
        tab.tabIndex = selected ? 0 : -1;
        $(`panel-${tab.dataset.tab}`).hidden = !selected;
    }
    emit('tab', name);
}

function select(path, range) {
    showTab('code');
    emit('select', { path, range });
}

for (const tab of tabs) {
    tab.addEventListener('click', () => showTab(tab.dataset.tab));
}

$('tablist').addEventListener('keydown', (event) => {
    const here = tabs.indexOf(document.activeElement);
    const moves = { ArrowRight: here + 1, ArrowLeft: here - 1, Home: 0, End: tabs.length - 1 };
    const go = moves[event.key];
    if (here < 0 || go === undefined) return;
    event.preventDefault();
    const tab = tabs[(go + tabs.length) % tabs.length];
    showTab(tab.dataset.tab);
    tab.focus();
});

showTab('code');

// starting up

async function start() {
    try {
        wasm = await boot();
        examples = JSON.parse(wasm.examples());
    } catch (error) {
        console.error(error);
        starting.textContent = "The console didn't start.";
        return;
    }
    for (const cart of examples) originals.set(cart.id, cart);
    const shared = await openFragment();
    renderPicker();
    const last = read(CURRENT);
    const remembered = last !== 'shared' && originals.has(last) ? last : null;
    const id = shared ? 'shared' : (remembered ?? examples[0]?.id ?? 'new');
    for (const control of document.querySelectorAll('.controls [disabled]')) {
        control.disabled = false;
    }
    openCart(id);
    await Promise.all(
        [
            ['./editor.js', 'mountEditor', 'mount-code'],
            ['./pixels.js', 'mountPixels', 'mount-sprites'],
        ].map(async ([path, name, id]) => {
            const el = $(id);
            try {
                const module = await import(path);
                await module[name](el, store);
            } catch (error) {
                console.error(error);
                el.textContent = `Couldn't load ${path}.`;
            }
        }),
    );
    starting.hidden = true;
}

start();
