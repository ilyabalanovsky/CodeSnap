<script lang="ts">
  import { tick } from 'svelte';
  import CodeEditor from './lib/CodeEditor.svelte';
  import MarkdownPreview from './lib/MarkdownPreview.svelte';
  import { detectLanguageFromCode, extensionForLanguage, languageOptions } from './lib/languages';
  import { copySnapshotPng, getClipboardText, hideToTray, saveSnapshotPng } from './lib/tauri';
  import { copyDataUrlToClipboard, renderNodeToPngDataUrl } from './lib/snapshot';

  type Backdrop = 'aurora' | 'graphite' | 'paper' | 'solid';
  type ExportState = 'idle' | 'saving' | 'copying' | 'saved' | 'copied' | 'failed';

  const sampleCode = `import { createSignal } from "solid-js";

type Snapshot = {
  language: "typescript" | "rust" | "python";
  scale: 2;
};

export function createCodeSnap(source: string): Snapshot {
  const [theme] = createSignal("darcula");
  return renderSnapshot(source, theme());
}`;

  const themes = [
    { value: 'darcula', label: 'Darcula' },
    { value: 'githubDark', label: 'GitHub Dark' },
    { value: 'githubLight', label: 'GitHub Light' },
    { value: 'materialDark', label: 'Material Dark' },
    { value: 'oneDark', label: 'One Dark' },
  ];

  const backgrounds = [
    { value: 'aurora', label: 'Aurora' },
    { value: 'graphite', label: 'Graphite' },
    { value: 'paper', label: 'Paper' },
    { value: 'solid', label: 'Solid' },
  ];

  let code = sampleCode;
  let languageChoice = '__auto';
  let detectedLanguage = detectLanguageFromCode(sampleCode);
  let editorTheme = 'darcula';
  let fileName = 'snippet.ts';
  let draftFileName = fileName;
  let isEditingFileName = false;
  let fileNameInput: HTMLInputElement;
  let lastLanguageExtension = 'ts';
  let backdrop: Backdrop = 'aurora';
  let includeBackground = true;
  let includeTitleBar = true;
  let onlyCode = false;
  let renderMarkdownPreview = true;
  let lineHeight = 1.55;
  let exportState: ExportState = 'idle';
  let stageNode: HTMLElement;
  let previewNode: HTMLElement;

  $: isBusy = exportState === 'saving' || exportState === 'copying';
  $: detectedLanguage = detectLanguageFromCode(code);
  $: activeLanguage = languageChoice === '__auto' ? detectedLanguage : languageChoice;
  $: isMarkdown = activeLanguage === 'Markdown';
  $: isMarkdownPreview = isMarkdown && renderMarkdownPreview;
  $: fileExtension = extensionForLanguage(activeLanguage);
  $: if (fileExtension !== lastLanguageExtension) {
    if (fileName === `snippet.${lastLanguageExtension}` || fileName === 'snippet') {
      fileName = `snippet.${fileExtension}`;
    }
    lastLanguageExtension = fileExtension;
  }
  $: actionLabel = {
    idle: '',
    saving: 'Saving...',
    copying: 'Copying...',
    saved: 'Saved',
    copied: 'Copied',
    failed: 'Failed',
  }[exportState];

  function updateCode(nextCode: string): void {
    code = nextCode;
  }

  function onLanguageChange(event: Event): void {
    languageChoice = (event.currentTarget as HTMLSelectElement).value;
  }

  function onOnlyCodeChange(event: Event): void {
    onlyCode = (event.currentTarget as HTMLInputElement).checked;

    if (onlyCode) {
      includeBackground = false;
      includeTitleBar = false;
    }
  }

  async function startFileNameEdit(): Promise<void> {
    draftFileName = fileName;
    isEditingFileName = true;
    await tick();
    fileNameInput.focus();
    fileNameInput.select();
  }

  function commitFileName(): void {
    const nextName = draftFileName.trim();
    fileName = nextName || 'snippet';
    isEditingFileName = false;
  }

  function cancelFileNameEdit(): void {
    draftFileName = fileName;
    isEditingFileName = false;
  }

  function onFileNameKeydown(event: KeyboardEvent): void {
    if (event.key === 'Enter') {
      event.preventDefault();
      commitFileName();
    }

    if (event.key === 'Escape') {
      event.preventDefault();
      cancelFileNameEdit();
    }
  }

  function imageFileName(): string {
    const safeName = fileName
      .trim()
      .replace(/\.[^.]+$/, '')
      .replace(/[\\/:*?"<>|]/g, '-')
      .replace(/\s+/g, ' ');

    return `${safeName || 'codesnap'}.png`;
  }

  async function loadClipboard(): Promise<void> {
    const text = await getClipboardText();
    if (text.trim()) {
      code = text;
      languageChoice = '__auto';
    }
  }

  async function savePng(): Promise<void> {
    await exportPng('save');
  }

  async function copyPng(): Promise<void> {
    await exportPng('copy');
  }

  async function exportPng(action: 'save' | 'copy'): Promise<void> {
    exportState = action === 'save' ? 'saving' : 'copying';
    await tick();

    try {
      const exportNode = includeBackground && !onlyCode ? stageNode : previewNode;
      const dataUrl = await renderNodeToPngDataUrl(exportNode);

      if (action === 'save') {
        const result = await saveSnapshotPng(dataUrl, imageFileName());
        exportState = result.saved ? 'saved' : 'idle';
      } else {
        const nativeCopied = await copySnapshotPng(dataUrl);
        const webCopied = nativeCopied || (await copyDataUrlToClipboard(dataUrl));
        exportState = webCopied ? 'copied' : 'failed';
      }

      if (exportState === 'saved' || exportState === 'copied') {
        window.setTimeout(() => void hideToTray(), 450);
      }
    } catch {
      exportState = 'failed';
    } finally {
      window.setTimeout(() => {
        if (exportState === 'saved' || exportState === 'copied' || exportState === 'failed') {
          exportState = 'idle';
        }
      }, 1600);
    }
  }
</script>

<svelte:head>
  <title>CodeSnap</title>
</svelte:head>

<main class="app-shell">
  <section bind:this={stageNode} class="stage" class:only-code={onlyCode} class:aurora={backdrop === 'aurora'} class:graphite={backdrop === 'graphite'} class:paper={backdrop === 'paper'} class:solid={backdrop === 'solid'} aria-label="Editable code snapshot">
    <article class="code-window" class:only-code={onlyCode} bind:this={previewNode}>
      {#if includeTitleBar && !onlyCode}
        <header class="window-bar">
        <div class="traffic" aria-hidden="true">
          <span></span>
          <span></span>
          <span></span>
        </div>
        {#if isEditingFileName}
          <input
            class="file-name-input"
            aria-label="File name"
            bind:this={fileNameInput}
            bind:value={draftFileName}
            on:blur={commitFileName}
            on:keydown={onFileNameKeydown}
          />
        {:else}
          <button class="file-label" type="button" on:click={startFileNameEdit}>{fileName || `snippet.${fileExtension}`}</button>
        {/if}
        </header>
      {/if}

      {#if isMarkdownPreview}
        <MarkdownPreview value={code} {lineHeight} />
      {:else}
        <CodeEditor value={code} language={activeLanguage} theme={editorTheme} {lineHeight} showLineNumbers={!onlyCode} onChange={updateCode} />
      {/if}
    </article>
  </section>

  <aside class="controls" aria-label="Snapshot settings">
    <div class="brand-row">
      <strong>CodeSnap</strong>
      {#if actionLabel}
        <span data-state={exportState}>{actionLabel}</span>
      {/if}
    </div>

    <div class="control-grid">
      <label>
        <span>Language</span>
        <select value={languageChoice} on:change={onLanguageChange}>
          <option value="__auto">Auto detect ({detectedLanguage})</option>
          {#each languageOptions as item}
            <option value={item.value}>{item.label}</option>
          {/each}
        </select>
        <small>Wrong language? Choose manually here.</small>
      </label>

      <label>
        <span>IDE theme</span>
        <select bind:value={editorTheme}>
          {#each themes as item}
            <option value={item.value}>{item.label}</option>
          {/each}
        </select>
      </label>
    </div>

    <label>
      <span>Background</span>
      <select bind:value={backdrop}>
        {#each backgrounds as item}
          <option value={item.value}>{item.label}</option>
        {/each}
      </select>
    </label>

    <label class="range-label">
      <span>Line height <strong>{lineHeight.toFixed(2)}</strong></span>
      <input type="range" min="1.2" max="2" step="0.05" bind:value={lineHeight} />
    </label>

    {#if isMarkdown}
      <label class="toggle-row">
        <input type="checkbox" bind:checked={renderMarkdownPreview} />
        <span>Markdown preview</span>
      </label>
    {/if}

    <label class="toggle-row">
      <input type="checkbox" checked={onlyCode} on:change={onOnlyCodeChange} />
      <span>Only code</span>
    </label>

    <label class="toggle-row" class:disabled={onlyCode}>
      <input type="checkbox" bind:checked={includeBackground} disabled={onlyCode} />
      <span>Include background</span>
    </label>

    <label class="toggle-row" class:disabled={onlyCode}>
      <input type="checkbox" bind:checked={includeTitleBar} disabled={onlyCode} />
      <span>Include title bar</span>
    </label>

    <div class="actions">
      <button class="secondary" type="button" on:click={loadClipboard}>Paste clipboard</button>
      <button class="primary" type="button" disabled={isBusy} on:click={savePng}>Save PNG</button>
      <button class="primary ghost" type="button" disabled={isBusy} on:click={copyPng}>Copy PNG</button>
    </div>
  </aside>
</main>
