// The code editor: CodeMirror with a small mimas mode, the file tabs on top and the list of
// problems underneath. Checking and running belong to app.js, which hears about every edit
// through `store.setFile`. The packages come from esm.sh through the import map in index.html.

import { EditorState } from '@codemirror/state';
import {
    EditorView,
    drawSelection,
    highlightActiveLine,
    highlightActiveLineGutter,
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
const NAME = /^[A-Za-z_][A-Za-z0-9_]*$/;
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
    // the files bar, with the buttons for adding a module and for renaming or deleting this one
    const fileList = h('div', { class: 'mb-filelist', role: 'group', 'aria-label': 'Files' });
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
    const bar = h('div', { class: 'mb-files' }, fileList, add, actions);

    // the form that names a new module or renames one
    const nameInput = h('input', {
        type: 'text',
        placeholder: 'module name',
        maxlength: 40,
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
    el.replaceChildren(h('div', { class: 'mb-editor' }, bar, nameBar, body, problems));

    let path = null;
    let naming = null;
    let applying = false;
    let armed = 0;
    let tabsKey = null;
    let problemsKey = null;
    const states = new Map();
    const buttons = new Map();

    const view = new EditorView({ parent: body });

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
            keymap.of([...defaultKeymap, ...historyKeymap, ...searchKeymap, indentWithTab]),
            theme,
            syntaxHighlighting(colors),
            EditorView.updateListener.of((update) => {
                if (update.docChanged && !applying) store.setFile(path, update.state.doc.toString());
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
        markProblems();
        renderTabs();
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

    // the files in the order of the tabs: the script, the modules, sprites.txt, the rest
    function ordered() {
        const files = store.cart.files;
        const rank = (file) =>
            isModule(file) ? 1 : file.endsWith('.mim') ? 0 : file === 'sprites.txt' ? 2 : 3;
        const ranks = new Map(Object.keys(files).map((file) => [file, rank(file)]));
        return [...ranks.keys()].sort(
            (a, b) => ranks.get(a) - ranks.get(b) || (a < b ? -1 : a > b ? 1 : 0),
        );
    }

    // a file the cart doesn't have (the one shown, while another cart opens) is not a module
    function isModule(file) {
        const text = store.cart.files[file ?? ''];
        return text !== undefined && file.endsWith('.mim') && store.isModule(text);
    }

    function fallback() {
        return ordered()[0] ?? null;
    }

    // the tabs and the problems

    function renderTabs() {
        const paths = ordered();
        const key = paths.join('\n');
        if (key !== tabsKey) {
            tabsKey = key;
            buttons.clear();
            for (const file of paths) {
                const tab = button(
                    file,
                    () => {
                        if (file !== path) show(file);
                        view.focus();
                    },
                    { class: 'mb-file' },
                );
                buttons.set(file, tab);
            }
            fileList.replaceChildren(...buttons.values());
        }
        const broken = new Set(store.diagnostics.map((d) => d.file));
        for (const [file, tab] of buttons) {
            tab.setAttribute('aria-pressed', file === path);
            tab.classList.toggle('has-error', broken.has(file));
            tab.title = broken.has(file) ? 'This file has problems' : '';
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
        if (!NAME.test(name)) {
            nameNote.textContent = 'Use letters, digits and underscores.';
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
        renderTabs();
    });

    store.on('diagnostics', () => {
        markProblems();
        renderTabs();
        renderProblems();
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
