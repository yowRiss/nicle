import { EditorState, Compartment, type Extension, Text } from '@codemirror/state';
import { EditorView, keymap, lineNumbers, highlightActiveLine, highlightActiveLineGutter, drawSelection } from '@codemirror/view';
import { defaultKeymap, history, historyKeymap, indentWithTab } from '@codemirror/commands';
import { bracketMatching, indentOnInput, syntaxHighlighting, HighlightStyle, indentUnit } from '@codemirror/language';
import { tags as t } from '@lezer/highlight';
import { search, searchKeymap, openSearchPanel } from '@codemirror/search';
import { api } from '../lib/api';

export type EditorSettings = { theme: 'dark' | 'light'; fontSize: number; tabSize: number; wordWrap: boolean; autoSave: boolean };
export type Tab = { path: string; dirty: boolean };
type Document = { state: EditorState; saved: Text; settings: Compartment; isLargeFile: boolean };

/**
 * Large-file mode threshold: files >= 500 KiB skip heavy syntax parsing/highlighting,
 * bracket matching, and indent-on-input, and restrict undo history to 10 events.
 */
export const LARGE_FILE_THRESHOLD_BYTES = 500 * 1024;
export const LARGE_FILE_HISTORY_DEPTH = 10;

const documents = new Map<string, Document>();
let view: EditorView | undefined;
let current = '';
let settings: EditorSettings = { theme: 'dark', fontSize: 14, tabSize: 2, wordWrap: false, autoSave: false };
let changed: (path: string, dirty: boolean, line: number, column: number) => void = () => {};
let wordWrapToggleCallback: (() => void) | undefined;

/**
 * Standard VS Code token syntax highlighting:
 * - Headings (##, ###): --syntax-heading, bold
 * - Comments (//, /* * /, #, <!-- -->): --syntax-comment, italic
 * - Strings ("", '', ``): --syntax-string
 * - Numbers, booleans, atoms: --syntax-number
 * - Keywords (const, let, fn, def, if, return): --syntax-keyword
 * - Functions & methods: --syntax-function
 * - Types, classes, namespaces: --syntax-type
 * - Variables & properties: --syntax-variable
 * - Tags (HTML/JSX): --syntax-tag
 * - Attributes: --syntax-attr
 * - Regular expressions: --syntax-regexp
 * - Punctuation & operators: --syntax-operator
 * - Invalid syntax: --error with underline
 */
const nicleHighlightStyle = HighlightStyle.define([
  { tag: [t.heading, t.heading1, t.heading2, t.heading3, t.heading4, t.heading5, t.heading6], color: 'var(--syntax-heading)', fontWeight: '600' },
  { tag: [t.comment, t.lineComment, t.blockComment, t.docComment], color: 'var(--syntax-comment)', fontStyle: 'italic' },
  { tag: [t.string, t.character, t.attributeValue, t.docString], color: 'var(--syntax-string)' },
  { tag: [t.number, t.integer, t.float, t.bool, t.null, t.atom], color: 'var(--syntax-number)' },
  { tag: [t.keyword, t.operatorKeyword, t.controlKeyword, t.definitionKeyword, t.moduleKeyword, t.modifier, t.self], color: 'var(--syntax-keyword)' },
  { tag: [t.function(t.variableName), t.function(t.propertyName), t.macroName], color: 'var(--syntax-function)' },
  { tag: [t.typeName, t.className, t.namespace], color: 'var(--syntax-type)' },
  { tag: [t.tagName], color: 'var(--syntax-tag)' },
  { tag: [t.attributeName], color: 'var(--syntax-attr)' },
  { tag: [t.regexp], color: 'var(--syntax-regexp)' },
  { tag: [t.variableName, t.propertyName], color: 'var(--syntax-variable)' },
  { tag: [t.operator, t.punctuation, t.separator, t.bracket, t.paren, t.brace, t.angleBracket], color: 'var(--syntax-operator)' },
  { tag: [t.emphasis], fontStyle: 'italic' },
  { tag: [t.strong], fontWeight: 'bold' },
  { tag: [t.link], color: 'var(--syntax-keyword)', textDecoration: 'underline' },
  { tag: [t.monospace], color: 'var(--syntax-string)' },
  { tag: [t.quote], color: 'var(--syntax-comment)', fontStyle: 'italic' },
  { tag: [t.list], color: 'var(--syntax-heading)' },
  { tag: [t.invalid], color: 'var(--error)', textDecoration: 'underline' },
]);

/**
 * Workbench unified editor theme honoring all dark/light variables
 */
const nicleEditorTheme = EditorView.theme({
  '&': {
    height: '100%',
    color: 'var(--text)',
    backgroundColor: 'var(--canvas)',
    fontFamily: "var(--font-mono, 'IBM Plex Mono', ui-monospace, monospace)",
  },
  '.cm-content': {
    padding: '12px 0',
    caretColor: 'var(--accent)',
  },
  '.cm-cursor, .cm-dropCursor': {
    borderLeftColor: 'var(--accent)',
    borderLeftWidth: '2px',
  },
  '&.cm-focused .cm-selectionBackground, .cm-selectionBackground, .cm-content ::selection': {
    backgroundColor: 'var(--selection) !important',
  },
  '.cm-activeLine': {
    backgroundColor: 'var(--sidebar)',
  },
  '.cm-gutters': {
    backgroundColor: 'var(--canvas)',
    color: 'var(--text-subtle)',
    border: 'none',
    borderRight: '1px solid var(--border)',
  },
  '.cm-activeLineGutter': {
    backgroundColor: 'var(--sidebar)',
    color: 'var(--text)',
  },
  '.cm-lineNumbers .cm-gutterElement': {
    padding: '0 12px 0 8px',
    minWidth: '32px',
    textAlign: 'right',
  },
  '.cm-matchingBracket, .cm-nonmatchingBracket': {
    backgroundColor: 'var(--hover)',
    outline: '1px solid var(--accent)',
    borderRadius: '2px',
  },
  '.cm-searchMatch': {
    backgroundColor: 'var(--selection)',
  },
  '.cm-searchMatch.cm-searchMatch-selected': {
    backgroundColor: 'var(--selection)',
    outline: '1px solid var(--warning)',
  },
  '.cm-panels': {
    backgroundColor: 'var(--sidebar)',
    color: 'var(--text)',
    borderBottom: '1px solid var(--border)',
    borderTop: '1px solid var(--border)',
    padding: '6px 12px',
  },
  '.cm-panels.cm-panels-top': {
    borderBottom: '1px solid var(--border)',
  },
  '.cm-panels.cm-panels-bottom': {
    borderTop: '1px solid var(--border)',
  },
  '.cm-search': {
    display: 'flex',
    flexWrap: 'wrap',
    alignItems: 'center',
    gap: '8px',
    padding: '4px 0',
  },
  '.cm-search input': {
    backgroundColor: 'var(--raised)',
    color: 'var(--text)',
    border: '1px solid var(--border)',
    borderRadius: '4px',
    padding: '4px 8px',
    fontSize: '12px',
    outline: 'none',
  },
  '.cm-search input:focus': {
    borderColor: 'var(--accent)',
    outline: '1px solid var(--accent)',
  },
  '.cm-button': {
    backgroundColor: 'var(--raised)',
    color: 'var(--text)',
    border: '1px solid var(--border)',
    borderRadius: '4px',
    padding: '4px 8px',
    fontSize: '12px',
    cursor: 'pointer',
  },
  '.cm-button:hover': {
    backgroundColor: 'var(--hover)',
  },
  '.cm-button:focus-visible': {
    outline: '2px solid var(--accent)',
  },
  '.cm-tooltip': {
    backgroundColor: 'var(--raised)',
    color: 'var(--text)',
    border: '1px solid var(--border)',
    borderRadius: '4px',
    boxShadow: '0 8px 24px var(--shadow)',
  },
  '.cm-scroller': {
    overflow: 'auto',
    fontFamily: 'inherit',
  },
  '.cm-line': {
    padding: '0 16px',
    lineHeight: '1.6',
  },
  '.cm-content.cm-lineWrapping': {
    wordBreak: 'break-word',
    overflowWrap: 'anywhere',
  },
  '.cm-lineWrapping .cm-line': {
    overflowWrap: 'anywhere',
    wordBreak: 'break-word',
  },
});

function configuration(isLargeFile = false): Extension[] {
  return [
    nicleEditorTheme,
    isLargeFile ? [] : syntaxHighlighting(nicleHighlightStyle, { fallback: true }),
    settings.wordWrap ? EditorView.lineWrapping : [],
    EditorState.tabSize.of(settings.tabSize),
    indentUnit.of(' '.repeat(settings.tabSize)),
    EditorView.theme({
      '&': { fontSize: `${settings.fontSize}px` },
    }),
  ];
}

async function language(path: string): Promise<Extension> {
  const ext = path.split('.').pop()?.toLowerCase();
  switch (ext) {
    case 'js': case 'jsx': case 'mjs': case 'cjs': case 'ts': case 'tsx': {
      const { javascript } = await import('@codemirror/lang-javascript');
      return javascript({ typescript: ext === 'ts' || ext === 'tsx', jsx: ext === 'jsx' || ext === 'tsx' });
    }
    case 'html': case 'htm': return (await import('@codemirror/lang-html')).html();
    case 'css': return (await import('@codemirror/lang-css')).css();
    case 'json': return (await import('@codemirror/lang-json')).json();
    case 'py': return (await import('@codemirror/lang-python')).python();
    case 'rs': return (await import('@codemirror/lang-rust')).rust();
    case 'c': case 'cpp': case 'h': case 'hpp': return (await import('@codemirror/lang-cpp')).cpp();
    case 'go': return (await import('@codemirror/lang-go')).go();
    case 'md': return (await import('@codemirror/lang-markdown')).markdown();
    default: return [];
  }
}

function createEditorKeymap() {
  return keymap.of([
    ...defaultKeymap,
    ...historyKeymap,
    ...searchKeymap,
    indentWithTab,
    {
      key: 'Alt-z',
      run: () => {
        if (wordWrapToggleCallback) {
          wordWrapToggleCallback();
          return true;
        }
        return false;
      },
    },
  ]);
}

export function mountEditor(element: HTMLElement, callback: typeof changed, onToggleWordWrap?: () => void) {
  changed = callback;
  wordWrapToggleCallback = onToggleWordWrap;
  view = new EditorView({ parent: element });
  return () => {
    view?.destroy();
    view = undefined;
    wordWrapToggleCallback = undefined;
  };
}

export async function openDocument(path: string) {
  if (!documents.has(path)) {
    const content = await api.read(path);
    const isLargeFile = content.length >= LARGE_FILE_THRESHOLD_BYTES;
    const syntax = isLargeFile ? [] : await language(path);
    const compartment = new Compartment();
    const state = EditorState.create({
      doc: content,
      extensions: [
        lineNumbers(),
        highlightActiveLine(),
        highlightActiveLineGutter(),
        drawSelection(),
        isLargeFile ? history({ minDepth: LARGE_FILE_HISTORY_DEPTH, newGroupDelay: 500 }) : history(),
        isLargeFile ? [] : bracketMatching(),
        isLargeFile ? [] : indentOnInput(),
        search(),
        createEditorKeymap(),
        syntax,
        compartment.of(configuration(isLargeFile)),
        EditorView.updateListener.of(update => {
          const document = documents.get(current);
          if (!document) return;
          document.state = update.state;
          const cursor = update.state.selection.main.head;
          const line = update.state.doc.lineAt(cursor);
          changed(current, !update.state.doc.eq(document.saved), line.number, cursor - line.from + 1);
        }),
      ],
    });
    documents.set(path, { state, saved: state.doc, settings: compartment, isLargeFile });
  }
  activateDocument(path);
}

export function activateDocument(path: string) {
  const document = documents.get(path);
  if (!document || !view) return;
  current = path;
  view.setState(document.state);
  view.dispatch({ effects: document.settings.reconfigure(configuration(document.isLargeFile)) });
  view.focus();
}

export async function saveDocument(path: string) {
  const document = documents.get(path);
  if (!document) return;
  const saved = document.state.doc;
  await api.save(path, saved.toString());
  document.saved = saved;
  changed(path, !document.state.doc.eq(saved), 0, 0);
}

export function resetEditorView() {
  current = '';
  if (view) {
    view.setState(EditorState.create({ doc: '' }));
  }
}

export function closeDocument(path: string) {
  documents.delete(path);
  if (current === path) {
    current = '';
    if (documents.size === 0 && view) {
      resetEditorView();
    }
  }
}

export function renameDocument(path: string, destination: string) {
  const doc = documents.get(path);
  if (doc) {
    documents.delete(path);
    documents.set(destination, doc);
    if (current === path) current = destination;
  }
}

export function configureEditor(next: EditorSettings) {
  settings = next;
  const doc = documents.get(current);
  if (view && doc) view.dispatch({ effects: doc.settings.reconfigure(configuration(doc.isLargeFile)) });
}

export function isLargeFileDocument(path: string): boolean {
  return documents.get(path)?.isLargeFile ?? false;
}

export async function toggleLargeFileMode(path?: string) {
  const targetPath = path || current;
  const doc = documents.get(targetPath);
  if (!doc) return;
  const nextIsLarge = !doc.isLargeFile;
  const content = doc.state.doc.toString();
  const syntax = nextIsLarge ? [] : await language(targetPath);
  const compartment = new Compartment();
  const nextState = EditorState.create({
    doc: content,
    extensions: [
      lineNumbers(),
      highlightActiveLine(),
      highlightActiveLineGutter(),
      drawSelection(),
      nextIsLarge ? history({ minDepth: LARGE_FILE_HISTORY_DEPTH, newGroupDelay: 500 }) : history(),
      nextIsLarge ? [] : bracketMatching(),
      nextIsLarge ? [] : indentOnInput(),
      search(),
      createEditorKeymap(),
      syntax,
      compartment.of(configuration(nextIsLarge)),
      EditorView.updateListener.of(update => {
        const document = documents.get(current);
        if (!document) return;
        document.state = update.state;
        const cursor = update.state.selection.main.head;
        const line = update.state.doc.lineAt(cursor);
        changed(current, !update.state.doc.eq(document.saved), line.number, cursor - line.from + 1);
      }),
    ],
  });
  doc.state = nextState;
  doc.saved = nextState.doc;
  doc.settings = compartment;
  doc.isLargeFile = nextIsLarge;
  if (current === targetPath && view) {
    view.setState(nextState);
    view.dispatch({ effects: compartment.reconfigure(configuration(nextIsLarge)) });
    view.focus();
  }
}

export function findInDocument(replace = false) {
  if (view) {
    openSearchPanel(view);
    if (replace) view.dom.querySelector<HTMLInputElement>('input[name=replace]')?.focus();
  }
}
