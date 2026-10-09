// The code editor: the cart's files as a tree on the left, CodeMirror with a small mimas mode
// and the list of problems on the right. Checking and running belong to app.js, which hears
// about every edit through `store.setFile`. The packages come from esm.sh through the import map
// in index.html.

import { EditorState, StateEffect, StateField } from '@codemirror/state';
import {
    Decoration,
    EditorView,
    WidgetType,
    drawSelection,
    highlightActiveLine,
    highlightActiveLineGutter,
    hoverTooltip,
    keymap,
    lineNumbers,
} from '@codemirror/view';
import { defaultKeymap, history, historyKeymap, indentWithTab } from '@codemirror/commands';
import {
    StreamLanguage,
    bracketMatching,
    indentOnInput,
    indentUnit,
    syntaxHighlighting,
} from '@codemirror/language';
import { lintGutter, setDiagnostics } from '@codemirror/lint';
import { highlightSelectionMatches, searchKeymap } from '@codemirror/search';
import { tagHighlighter, tags } from '@lezer/highlight';
import { button, h } from './dom.js';

const MODULE = 'module @;\n';
// a module's name, with a folder for each part before a slash
const NAME = /^[A-Za-z_][A-Za-z0-9_]*(?:\/[A-Za-z_][A-Za-z0-9_]*)*$/;
const MAX_ROWS = 50;

const KEYWORDS = new Set([
    ...['let', 'const', 'fn', 'struct', 'enum', 'pact', 'impl', 'pub', 'use', 'module'],
    ...['if', 'else', 'match', 'for', 'while', 'loop', 'in', 'break', 'continue', 'return'],
    ...['raise', 'absolve', 'collect', 'self', 'Self'],
]);

const mimas = StreamLanguage.define({
    name: 'mimas',
    token(stream) {
        if (stream.match('//')) {
            stream.skipToEnd();
            return 'comment';
        }
        if (stream.match(/^f?"/)) {
            while (!stream.eol()) {
                const c = stream.next();
                if (c === '\\') stream.next();
                else if (c === '"') break;
            }
            return 'string';
        }
        if (stream.match(/^\d[\d_]*(?:\.\d[\d_]*)?/)) return 'number';
        if (stream.match(/^[A-Za-z_]\w*/)) {
            const word = stream.current();
            if (KEYWORDS.has(word)) return 'keyword';
            if (word === 'true' || word === 'false' || word === 'null') return 'atom';
            if (/^[A-Z]/.test(word)) return 'typeName';
            return stream.match(/^\s*\(/, false) ? 'variableName.function' : 'variableName';
        }
        stream.next();
        return null;
    },
    languageData: { commentTokens: { line: '//' } },
});

// the classes the book's highlight-tokyo-night.css colors
const colors = tagHighlighter([
    { tag: tags.keyword, class: 'hljs-keyword' },
    { tag: tags.comment, class: 'hljs-comment' },
    { tag: tags.string, class: 'hljs-string' },
    { tag: tags.number, class: 'hljs-number' },
    { tag: tags.atom, class: 'hljs-literal' },
    { tag: tags.typeName, class: 'hljs-title' },
    { tag: tags.function(tags.variableName), class: 'hljs-title' },
]);

const theme = EditorView.theme(
    {
        '&': { height: '100%', backgroundColor: 'var(--ground)', color: '#a9b1d6', fontSize: '13px' },
        '.cm-scroller': { fontFamily: 'var(--mono)', lineHeight: '1.55' },
        '.cm-content': { caretColor: '#c0caf5', padding: '8px 0' },
        '.cm-cursor, .cm-dropCursor': { borderLeftColor: '#c0caf5' },
        '&.cm-focused > .cm-scroller > .cm-selectionLayer .cm-selectionBackground, .cm-selectionBackground':
            { backgroundColor: 'rgb(65 166 246 / 0.35)' },
        '.cm-activeLine, .cm-activeLineGutter': { backgroundColor: 'rgb(255 255 255 / 0.04)' },
        '.cm-gutters': {
            backgroundColor: 'var(--ground)',
            color: 'var(--slate)',
            borderRight: '1px solid var(--surface)',
        },
        '.cm-activeLineGutter': { color: 'var(--muted)' },
        '.cm-matchingBracket, &.cm-focused .cm-matchingBracket': {
            backgroundColor: 'rgb(255 255 255 / 0.12)',
        },
        '.cm-searchMatch': { backgroundColor: 'rgb(255 205 117 / 0.3)' },
        '.cm-selectionMatch': { backgroundColor: 'rgb(255 255 255 / 0.08)' },
        '.cm-lintRange-error, .cm-lintRange-warning': {
            backgroundImage: 'none',
            textDecorationLine: 'underline',
            textDecorationStyle: 'wavy',
            textDecorationThickness: '1px',
            textUnderlineOffset: '3px',
        },
        '.cm-lintRange-error': { textDecorationColor: '#f7768e' },
        '.cm-lintRange-warning': { textDecorationColor: 'var(--orange)' },
        '.cm-tooltip': {
            backgroundColor: 'var(--surface)',
            border: '1px solid var(--line)',
            color: 'var(--text)',
        },
        '.cm-panels': { backgroundColor: 'var(--surface)', color: 'var(--text)' },
        '.cm-panels.cm-panels-bottom': { borderTop: '1px solid var(--line)' },
        '.cm-panel input, .cm-panel button': { font: 'inherit' },
    },
    { dark: true },
);

export function mountEditor(el, store) {
    // the file tree, with the buttons for adding a module and for renaming or deleting this one
    const tree = h('div', { class: 'mb-tree', role: 'group', 'aria-label': 'Files' });
    const add = button('+', () => openName('add'), { class: 'mb-add', title: 'Add a module' });
    const rename = button('Rename', () => openName('rename'));
    const remove = button('Delete', () => {
        if (remove.classList.contains('armed')) {
            disarm();
            deleteModule();
            return;
        }
        remove.classList.add('armed');
        remove.textContent = 'Click again to delete';
        armed = setTimeout(disarm, 3000);
    });
    remove.addEventListener('blur', disarm);
    const actions = h('div', { class: 'mb-fileactions' }, rename, remove);

    // the form that names a new module or renames one
    const nameInput = h('input', {
        type: 'text',
        placeholder: 'name, like enemies/boss',
        maxlength: 80,
        spellcheck: 'false',
        autocomplete: 'off',
        'aria-label': 'Module name',
    });
    const nameSubmit = button('Add', () => {}, { type: 'submit' });
    const nameNote = h('span', { class: 'mb-namenote', role: 'status' });
    const nameBar = h(
        'form',
        { class: 'mb-namebar', hidden: true },
        nameInput,
        nameSubmit,
        button('Cancel', cancelName),
        nameNote,
    );
    nameBar.addEventListener('submit', (event) => {
        event.preventDefault();
        submitName();
    });
    nameInput.addEventListener('keydown', (event) => {
        if (event.key !== 'Escape') return;
        event.stopPropagation();
        cancelName();
    });

    const body = h('div', { class: 'mb-body' });
    const problems = h('div', {
        class: 'mb-problems',
        role: 'region',
        'aria-label': 'Problems',
        hidden: true,
    });
    el.classList.add('mb-host');
    const side = h(
        'div',
        { class: 'mb-side' },
        h('div', { class: 'mb-actions' }, add, actions),
        nameBar,
        tree,
    );
    const main = h('div', { class: 'mb-main' }, body, problems);
    el.replaceChildren(h('div', { class: 'mb-editor' }, side, main));

    let path = null;
    let naming = null;
    let applying = false;
    let armed = 0;
    let treeKey = null;
    let problemsKey = null;
    const states = new Map();
    const buttons = new Map();
    const folders = new Map();
    const closed = new Set();

    const view = new EditorView({ parent: body });

    // what the language server's analysis says about the file shown, asked through the wasm
    // module in the protocol's positions (0-based lines, UTF-16 columns)
    function ask(question, ...args) {
        if (!store.lsp || path === null) return null;
        try {
            const json = store.lsp[question](path, ...args);
            return json === undefined || json === null ? null : JSON.parse(json);
        } catch (error) {
            console.error(error);
            return null;
        }
    }

    function refreshHints() {
        const doc = view.state.doc;
        const list = ask('inlay_hints') ?? [];
        const widgets = list.map((hint) =>
            Decoration.widget({ widget: new Hint(hint.label), side: 1 }).range(at(doc, hint.position)),
        );
        view.dispatch({ effects: hints.set.of(Decoration.set(widgets, true)) });
    }

    function refreshRefs() {
        const doc = view.state.doc;
        const list = view.hasFocus ? (ask('highlights', ...cursorPosition(view.state)) ?? []) : [];
        const marks = list
            .map((range) => rangeOf(doc, range))
            .filter(([from, to]) => from < to)
            .map(([from, to]) => Decoration.mark({ class: 'mb-ref' }).range(from, to));
        view.dispatch({ effects: refs.set.of(Decoration.set(marks, true)) });
    }

    const hover = hoverTooltip(
        (view, pos) => {
            const doc = view.state.doc;
            const result = ask('hover', ...position(doc, pos));
            if (!result) return null;
            const [from, to] = result.range ? rangeOf(doc, result.range) : [pos, pos];
            return { pos: from, end: to, above: true, create: () => ({ dom: tooltip(result.contents.value) }) };
        },
        { hoverTime: 250 },
    );

    // Ctrl-click (or Cmd-click) goes to where the name is declared
    const jump = EditorView.domEventHandlers({
        mousedown(event, view) {
            if (!(event.ctrlKey || event.metaKey) || event.button !== 0) return false;
            const pos = view.posAtCoords({ x: event.clientX, y: event.clientY });
            if (pos === null) return false;
            const place = ask('definition', ...position(view.state.doc, pos));
            if (!place) return false;
            event.preventDefault();
            store.select(place.file, selectable(place.range));
            return true;
        },
    });

    function renameAt(view) {
        if (!store.ensureAnalysis()) return false;
        const doc = view.state.doc;
        const where = cursorPosition(view.state);
        const range = ask('prepare_rename', ...where);
        if (!range) return false;
        const [from, to] = rangeOf(doc, range);
        const old = doc.sliceString(from, to);
        const name = window.prompt(`Rename ${old} to`, old);
        if (!name || name === old) return true;
        const result = ask('rename', ...where, name);
        if (!result) return true;
        if (result.error) {
            store.notify(result.error);
            return true;
        }
        for (const [file, edits] of Object.entries(result.changes)) {
            if (file === path) {
                view.dispatch({
                    changes: edits.map((edit) => {
                        const [from, to] = rangeOf(doc, edit.range);
                        return { from, to, insert: edit.newText };
                    }),
                });
            } else {
                store.setFile(file, edited(store.cart.files[file], edits));
            }
        }
        return true;
    }

    let refTimer = 0;

    function makeState(file, text) {
        const extensions = [
            lineNumbers(),
            highlightActiveLineGutter(),
            history(),
            drawSelection(),
            indentUnit.of('    '),
            EditorState.tabSize.of(4),
            bracketMatching(),
            indentOnInput(),
            highlightActiveLine(),
            highlightSelectionMatches(),
            lintGutter(),
            keymap.of([
                ...defaultKeymap,
                ...historyKeymap,
                ...searchKeymap,
                indentWithTab,
                { key: 'F2', run: renameAt },
            ]),
            theme,
            syntaxHighlighting(colors),
            hover,
            jump,
            refs.field,
            hints.field,
            EditorView.updateListener.of((update) => {
                if (update.docChanged && !applying) store.setFile(path, update.state.doc.toString());
                if (update.selectionSet || update.focusChanged) {
                    clearTimeout(refTimer);
                    refTimer = setTimeout(refreshRefs, 60);
                }
            }),
        ];
        if (file.endsWith('.mim')) extensions.push(mimas);
        return EditorState.create({ doc: text, extensions });
    }

    // switching files and keeping the text in step with the store

    function show(next) {
        if (path !== null) states.set(path, view.state);
        path = next;
        disarm();
        if (naming) closeName();
        const text = path === null ? '' : store.cart.files[path];
        view.setState(states.get(path) ?? makeState(path ?? '', text));
        view.contentDOM.setAttribute('aria-label', path === null ? 'Code' : `Code of ${path}`);
        view.contentDOM.contentEditable = path === null ? 'false' : 'true';
        // the folders on the way to the file open up
        for (let cut = (path ?? '').indexOf('/'); cut > 0; cut = path.indexOf('/', cut + 1)) {
            closed.delete(path.slice(0, cut + 1));
        }
        markProblems();
        renderTree();
        refreshHints();
        refreshRefs();
    }

    // an edit that came from somewhere else (the sprite editor writes sprites.txt)
    function setText(text) {
        applying = true;
        try {
            view.dispatch({ changes: { from: 0, to: view.state.doc.length, insert: text } });
        } finally {
            applying = false;
        }
    }

    // the files in the order of the tree, which is alphabetical
    function ordered() {
        return Object.keys(store.cart.files).sort();
    }

    // the folders directly under `prefix`, each a `details` holding its own branch, then the
    // files there
    function branch(paths, prefix) {
        const inside = paths.filter((file) => file.startsWith(prefix));
        const dirs = inside
            .map((file) => file.slice(prefix.length).split('/'))
            .filter((parts) => parts.length > 1)
            .map((parts) => parts[0]);
        const nodes = [...new Set(dirs)].sort().map((name) => {
            const dir = `${prefix}${name}/`;
            const details = h(
                'details',
                { class: 'mb-folder', open: !closed.has(dir) },
                h('summary', {}, name),
                ...branch(inside, dir),
            );
            details.addEventListener('toggle', () => {
                if (details.open) closed.delete(dir);
                else closed.add(dir);
            });
            folders.set(dir, details);
            return details;
        });
        for (const file of inside) {
            const name = file.slice(prefix.length);
            if (name.includes('/')) continue;
            const tab = button(
                name,
                () => {
                    if (file !== path) show(file);
                    view.focus();
                },
                { class: 'mb-file' },
            );
            buttons.set(file, tab);
            nodes.push(tab);
        }
        return nodes;
    }

    // a file the cart doesn't have (the one shown, while another cart opens) is not a module
    function isModule(file) {
        const text = store.cart.files[file ?? ''];
        return text !== undefined && file.endsWith('.mim') && store.isModule(text);
    }

    // the cart's script, which is what opens when nothing else is showing
    function fallback() {
        const paths = ordered();
        return paths.find((file) => file.endsWith('.mim') && !isModule(file)) ?? paths[0] ?? null;
    }

    // the tree and the problems

    function renderTree() {
        const paths = ordered();
        const key = paths.join('\n');
        if (key !== treeKey) {
            treeKey = key;
            buttons.clear();
            folders.clear();
            tree.replaceChildren(...branch(paths, ''));
        }
        const broken = new Set(store.diagnostics.map((d) => d.file));
        for (const [file, tab] of buttons) {
            tab.setAttribute('aria-pressed', file === path);
            tab.classList.toggle('has-error', broken.has(file));
            tab.title = broken.has(file) ? `${file} has problems` : file;
        }
        for (const [dir, details] of folders) {
            details.open = !closed.has(dir);
            details.classList.toggle(
                'has-error',
                [...broken].some((file) => file.startsWith(dir)),
            );
        }
        actions.hidden = !isModule(path);
    }

    function markProblems() {
        const doc = view.state.doc;
        const list = [];
        for (const diagnostic of store.diagnostics) {
            if (diagnostic.file !== path) continue;
            const from = offset(doc, diagnostic.line, diagnostic.col);
            const end = offset(doc, diagnostic.end_line ?? diagnostic.line, diagnostic.end_col ?? 0);
            const notes = [diagnostic.label, diagnostic.help].filter((n) => n && n !== diagnostic.message);
            list.push({
                from,
                to: end > from ? end : Math.min(from + 1, doc.length),
                severity: diagnostic.kind === 'fault' ? 'warning' : 'error',
                message: diagnostic.message,
                renderMessage: () =>
                    h('div', {}, diagnostic.message, ...notes.map((note) => h('div', { class: 'mb-note' }, note))),
            });
        }
        view.dispatch(setDiagnostics(view.state, list));
    }

    function renderProblems() {
        const list = store.diagnostics;
        const key = JSON.stringify(list);
        if (key === problemsKey) return;
        problemsKey = key;
        problems.hidden = list.length === 0;
        const rows = list.slice(0, MAX_ROWS).map((diagnostic) => {
            const { file, line, col, message, label, help, kind } = diagnostic;
            const notes = [label, help].filter((note) => note && note !== message);
            const pick = button(
                '',
                () => store.select(file, diagnostic),
                { class: 'mb-problem', 'data-kind': kind === 'fault' ? 'fault' : 'error' },
            );
            if (file) pick.append(h('span', { class: 'mb-where' }, `${file}:${line}:${col}`));
            pick.append(
                h('span', { class: 'mb-message' }, message),
                ...notes.map((note) => h('span', { class: 'mb-note' }, note)),
            );
            return h('li', {}, pick);
        });
        problems.replaceChildren(h('ul', {}, ...rows));
        if (list.length > MAX_ROWS) {
            problems.append(h('p', { class: 'mb-more' }, `and ${list.length - MAX_ROWS} more`));
        }
    }

    // adding, renaming and deleting modules

    function openName(mode) {
        naming = mode;
        nameBar.hidden = false;
        nameInput.value = mode === 'rename' ? path.replace(/\.mim$/, '') : '';
        nameSubmit.textContent = mode === 'rename' ? 'Rename' : 'Add';
        nameNote.textContent = '';
        nameInput.focus();
        nameInput.select();
    }

    function closeName() {
        naming = null;
        nameBar.hidden = true;
    }

    function cancelName() {
        closeName();
        if (path !== null) view.focus();
    }

    function disarm() {
        clearTimeout(armed);
        remove.classList.remove('armed');
        remove.textContent = 'Delete';
    }

    function submitName() {
        const name = nameInput.value.trim().replace(/\.mim$/, '');
        const target = `${name}.mim`;
        const files = store.cart.files;
        if (naming === 'rename' && target === path) {
            closeName();
            return;
        }
        // a cart is only files, so a folder is there once a file is in it
        if (name.endsWith('/') && NAME.test(name.slice(0, -1))) {
            nameNote.textContent = `A folder comes with its first file. Name that too, like ${name}main.`;
            return;
        }
        if (!NAME.test(name)) {
            nameNote.textContent = 'Use letters, digits and underscores, with / for a folder.';
            return;
        }
        if (Object.hasOwn(files, target)) {
            nameNote.textContent = 'That file exists.';
            return;
        }
        if (naming === 'add') {
            store.setFile(target, MODULE);
            show(target);
        } else {
            const old = path;
            store.setFile(target, files[old]);
            show(target);
            store.setFile(old, null);
            states.delete(old);
        }
        closeName();
        view.focus();
    }

    function deleteModule() {
        const gone = path;
        show(fallback());
        store.setFile(gone, null);
        states.delete(gone);
        view.focus();
    }

    store.on('cart', () => {
        const files = store.cart.files;
        const keep = path !== null && Object.hasOwn(files, path) ? path : fallback();
        states.clear();
        path = null;
        show(keep);
        renderProblems();
    });

    store.on('file', (changed) => {
        const files = store.cart.files;
        if (changed === path && !Object.hasOwn(files, path)) {
            show(fallback());
        } else if (changed === path && files[path] !== view.state.doc.toString()) {
            setText(files[path]);
        } else if (changed !== path) {
            states.delete(changed);
        }
        renderTree();
    });

    // a check has run, so the analysis is current again
    store.on('diagnostics', () => {
        markProblems();
        renderTree();
        renderProblems();
        refreshHints();
        refreshRefs();
    });

    store.on('select', ({ path: target, range }) => {
        if (!Object.hasOwn(store.cart.files, target)) return;
        if (target !== path) show(target);
        if (range) {
            const doc = view.state.doc;
            const anchor = offset(doc, range.line, range.col);
            const head = Math.max(anchor, offset(doc, range.end_line ?? range.line, range.end_col ?? 0));
            view.dispatch({
                selection: { anchor, head },
                effects: EditorView.scrollIntoView(anchor, { y: 'center' }),
            });
        }
        view.focus();
    });

    show(fallback());
    renderProblems();
}

// the offset of a 1 based line and column, kept inside the document
function offset(doc, line, col) {
    const at = doc.line(Math.min(Math.max(line, 1), doc.lines));
    return Math.min(at.from + Math.max(col - 1, 0), at.to);
}

// the protocol's positions, 0 based with UTF-16 columns, against the document

function at(doc, { line, character }) {
    const row = doc.line(Math.min(line + 1, doc.lines));
    return Math.min(row.from + character, row.to);
}

function position(doc, offset) {
    const row = doc.lineAt(offset);
    return [row.number - 1, offset - row.from];
}

// the position of the caret, or of the character before it when the caret sits at the end of a
// word, since that is the word someone means
function cursorPosition(state) {
    const { head } = state.selection.main;
    const before = head > 0 && /\w/.test(state.doc.sliceString(head - 1, head));
    const after = /\w/.test(state.doc.sliceString(head, head + 1));
    return position(state.doc, before && !after ? head - 1 : head);
}

function rangeOf(doc, range) {
    return [at(doc, range.start), at(doc, range.end)];
}

// a range in the shape of a diagnostic, which is what store.select takes
function selectable(range) {
    return {
        line: range.start.line + 1,
        col: range.start.character + 1,
        end_line: range.end.line + 1,
        end_col: range.end.character + 1,
    };
}

// `text` with the protocol's edits applied, back to front so the earlier ones keep their offsets
function edited(text, edits) {
    const starts = [0];
    for (let i = 0; i < text.length; i += 1) {
        if (text[i] === '\n') starts.push(i + 1);
    }
    const offsetOf = ({ line, character }) =>
        Math.min((starts[line] ?? text.length) + character, text.length);
    return [...edits]
        .sort((a, b) => offsetOf(b.range.start) - offsetOf(a.range.start))
        .reduce(
            (out, edit) =>
                out.slice(0, offsetOf(edit.range.start)) + edit.newText + out.slice(offsetOf(edit.range.end)),
            text,
        );
}

// a set of decorations the editor replaces whole, as a field and the effect that sets it
function decorations() {
    const set = StateEffect.define();
    const field = StateField.define({
        create: () => Decoration.none,
        update(value, transaction) {
            value = value.map(transaction.changes);
            for (const effect of transaction.effects) {
                if (effect.is(set)) value = effect.value;
            }
            return value;
        },
        provide: (field) => EditorView.decorations.from(field),
    });
    return { set, field };
}

const refs = decorations();
const hints = decorations();

class Hint extends WidgetType {
    constructor(label) {
        super();
        this.label = label;
    }

    eq(other) {
        return other.label === this.label;
    }

    toDOM() {
        return h('span', { class: 'mb-hint' }, this.label);
    }

    ignoreEvent() {
        return true;
    }
}

// the hover's markdown, as far as the docs go: fenced blocks, rules, paragraphs and code spans
function tooltip(markdown) {
    const root = h('div', { class: 'mb-tip' });
    for (const block of markdown.split(/\n{2,}/)) {
        const text = block.trim();
        const fence = /^```\w*\n([\s\S]*?)\n?```$/.exec(text);
        if (fence) root.append(h('pre', {}, h('code', {}, fence[1])));
        else if (text === '---') root.append(h('hr'));
        else if (text) {
            const parts = text.split(/(`[^`]+`)/).map((part) =>
                part.startsWith('`') && part.endsWith('`') ? h('code', {}, part.slice(1, -1)) : part,
            );
            root.append(h('p', {}, ...parts));
        }
    }
    return root;
}
