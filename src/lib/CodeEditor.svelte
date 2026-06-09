<script lang="ts">
  import { onDestroy, onMount } from 'svelte';
  import { Compartment, EditorState, type Extension } from '@codemirror/state';
  import { defaultKeymap, history, historyKeymap } from '@codemirror/commands';
  import { bracketMatching, defaultHighlightStyle, foldGutter, foldKeymap, indentOnInput, syntaxHighlighting } from '@codemirror/language';
  import { closeBrackets, closeBracketsKeymap, completionKeymap } from '@codemirror/autocomplete';
  import { highlightSelectionMatches, searchKeymap } from '@codemirror/search';
  import { EditorView, drawSelection, dropCursor, highlightSpecialChars, keymap, lineNumbers, rectangularSelection } from '@codemirror/view';
  import { languages as codeMirrorLanguages } from '@codemirror/language-data';
  import { oneDark } from '@codemirror/theme-one-dark';
  import { atomone } from '@uiw/codemirror-theme-atomone';
  import { darcula } from '@uiw/codemirror-theme-darcula';
  import { dracula } from '@uiw/codemirror-theme-dracula';
  import { githubDark, githubLight } from '@uiw/codemirror-theme-github';
  import { materialDark } from '@uiw/codemirror-theme-material';
  import { monokai } from '@uiw/codemirror-theme-monokai';
  import { nord } from '@uiw/codemirror-theme-nord';
  import { quietlight } from '@uiw/codemirror-theme-quietlight';
  import { solarizedDark, solarizedLight } from '@uiw/codemirror-theme-solarized';
  import { sublime } from '@uiw/codemirror-theme-sublime';
  import { tokyoNight } from '@uiw/codemirror-theme-tokyo-night';
  import { vscodeDark } from '@uiw/codemirror-theme-vscode';
  import { xcodeDark, xcodeLight } from '@uiw/codemirror-theme-xcode';

  export let value = '';
  export let language = 'typescript';
  export let theme = 'darcula';
  export let lineHeight = 1.55;
  export let showLineNumbers = true;
  export let onChange: (value: string) => void = () => {};

  let host: HTMLDivElement;
  let view: EditorView | undefined;
  let internalValue = value;
  let languageLoadId = 0;

  const languageCompartment = new Compartment();
  const setupCompartment = new Compartment();
  const themeCompartment = new Compartment();
  const lineHeightCompartment = new Compartment();

  const themeExtensions: Record<string, Extension> = {
    atomone,
    darcula,
    dracula,
    githubDark,
    githubLight,
    materialDark,
    monokai,
    nord,
    oneDark,
    quietlight,
    solarizedDark,
    solarizedLight,
    sublime,
    tokyoNight,
    vscodeDark,
    xcodeDark,
    xcodeLight,
  };

  function editorSetup(withLineNumbers: boolean): Extension {
    return [
    ...(withLineNumbers ? [lineNumbers(), foldGutter()] : []),
    highlightSpecialChars(),
    history(),
    drawSelection(),
    dropCursor(),
    indentOnInput(),
    syntaxHighlighting(defaultHighlightStyle, { fallback: true }),
    bracketMatching(),
    closeBrackets(),
    rectangularSelection(),
    highlightSelectionMatches(),
    keymap.of([
      ...closeBracketsKeymap,
      ...defaultKeymap,
      ...searchKeymap,
      ...historyKeymap,
      ...foldKeymap,
      ...completionKeymap,
    ]),
    ];
  }

  async function applyLanguage(name: string): Promise<void> {
    if (!view) {
      return;
    }

    const loadId = ++languageLoadId;
    const description = codeMirrorLanguages.find((item) => item.name === name);
    const extension = description ? await description.load().catch(() => []) : [];

    if (view && loadId === languageLoadId) {
      view.dispatch({ effects: languageCompartment.reconfigure(extension) });
    }
  }

  function lineHeightExtension(height: number): Extension {
    return EditorView.theme({
      '&': {
        fontSize: '14px',
        height: 'auto',
      },
      '.cm-content': {
        fontFamily: '"JetBrains Mono", "SFMono-Regular", Consolas, "Liberation Mono", monospace',
        lineHeight: String(height),
        padding: '28px 30px 76px',
      },
      '.cm-line': {
        padding: '0',
      },
      '.cm-scroller': {
        fontFamily: '"JetBrains Mono", "SFMono-Regular", Consolas, "Liberation Mono", monospace',
        overflow: 'visible',
      },
      '.cm-gutters': {
        borderRight: '0',
      },
      '.cm-focused': {
        outline: 'none',
      },
    });
  }

  onMount(() => {
    view = new EditorView({
      parent: host,
      state: EditorState.create({
        doc: value,
        extensions: [
          setupCompartment.of(editorSetup(showLineNumbers)),
          languageCompartment.of([]),
          themeCompartment.of(themeExtensions[theme] ?? darcula),
          lineHeightCompartment.of(lineHeightExtension(lineHeight)),
          EditorView.lineWrapping,
          EditorView.updateListener.of((update) => {
            if (!update.docChanged) {
              return;
            }

            internalValue = update.state.doc.toString();
            onChange(internalValue);
          }),
        ],
      }),
    });
    void applyLanguage(language);
  });

  onDestroy(() => {
    view?.destroy();
  });

  $: if (view && value !== internalValue) {
    view.dispatch({
      changes: {
        from: 0,
        to: view.state.doc.length,
        insert: value,
      },
    });
    internalValue = value;
  }

  $: if (view) {
    void applyLanguage(language);
  }

  $: if (view) {
    view.dispatch({ effects: setupCompartment.reconfigure(editorSetup(showLineNumbers)) });
  }

  $: if (view) {
    view.dispatch({ effects: themeCompartment.reconfigure(themeExtensions[theme] ?? darcula) });
  }

  $: if (view) {
    view.dispatch({ effects: lineHeightCompartment.reconfigure(lineHeightExtension(lineHeight)) });
  }
</script>

<div class="editor-host" bind:this={host}></div>
