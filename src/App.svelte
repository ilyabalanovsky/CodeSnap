<script lang="ts">
  import { Clipboard, Copy, Save } from '@lucide/svelte';
  import { tick } from 'svelte';
  import CodeEditor from './lib/CodeEditor.svelte';
  import CustomSelect from './lib/CustomSelect.svelte';
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
  let previewMotionKey = 0;
  let backgroundMotionKey = 0;
  let backgroundFlashMode: 'in' | 'out' | null = 'in';
  let backgroundFlashBackdrop: Backdrop = backdrop;
  let backgroundFlashTimer: number | undefined;

  $: isBusy = exportState === 'saving' || exportState === 'copying';
  $: detectedLanguage = detectLanguageFromCode(code);
  $: languageSelectOptions = [{ value: '__auto', label: `Auto detect (${detectedLanguage})` }, ...languageOptions];
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

  function cuePreviewMotion(): void {
    previewMotionKey += 1;
  }

  function cueBackgroundMotion(direction: 'in' | 'out' = 'in', motionBackdrop: Backdrop = backdrop): void {
    if (backgroundFlashTimer) {
      window.clearTimeout(backgroundFlashTimer);
    }

    backgroundFlashBackdrop = motionBackdrop;
    backgroundFlashMode = direction;
    backgroundMotionKey += 1;

    backgroundFlashTimer = window.setTimeout(() => {
      backgroundFlashMode = null;
      backgroundMotionKey += 1;
    }, 760);
  }

  function clamp(value: number, min = 0, max = 1): number {
    return Math.min(max, Math.max(min, value));
  }

  function easeOutCubic(t: number): number {
    return 1 - Math.pow(1 - t, 3);
  }

  function easeOutBack(t: number): number {
    const c1 = 1.56;
    const c3 = c1 + 1;

    return 1 + c3 * Math.pow(t - 1, 3) + c1 * Math.pow(t - 1, 2);
  }

  function drawDownProgress(t: number): { opacity: number; translateY: number; scaleY: number; height: number; blur: number; shadow: number } {
    if (t < 0.62) {
      const progress = easeOutBack(t / 0.62);
      const heightProgress = easeOutCubic(t / 0.62);

      return {
        opacity: clamp(progress),
        translateY: -18 + 21 * progress,
        scaleY: 0.72 + 0.36 * progress,
        height: heightProgress,
        blur: 5 * (1 - clamp(heightProgress)),
        shadow: clamp(progress),
      };
    }

    const progress = easeOutCubic((t - 0.62) / 0.38);

    return {
      opacity: 1,
      translateY: 3 - 3 * progress,
      scaleY: 1.08 - 0.08 * progress,
      height: 1,
      blur: 0,
      shadow: 1 - 0.2 * progress,
    };
  }

  function terminalBar(node: Element): { duration: number; css: (t: number) => string } {
    const height = node.getBoundingClientRect().height || 42;

    return {
      duration: 460,
      css: (t) => {
        const frame = drawDownProgress(t);

        return `
          min-height: ${height * frame.height}px;
          max-height: ${height * frame.height}px;
          opacity: ${frame.opacity};
          transform: translateY(${frame.translateY}px) scaleY(${frame.scaleY});
          filter: blur(${frame.blur}px);
          box-shadow: inset 0 -1px 0 rgba(255, 255, 255, ${0.08 * frame.shadow});
          overflow: hidden;
        `;
      },
    };
  }

  function onLanguageChange(nextLanguage: string): void {
    languageChoice = nextLanguage;
    cuePreviewMotion();
  }

  function onThemeChange(nextTheme: string): void {
    editorTheme = nextTheme;
    cuePreviewMotion();
  }

  function onBackdropChange(nextBackdrop: Backdrop): void {
    backdrop = nextBackdrop;
    cueBackgroundMotion('in', nextBackdrop);
  }

  function onOnlyCodeChange(event: Event): void {
    const nextOnlyCode = (event.currentTarget as HTMLInputElement).checked;
    const backgroundDirection = nextOnlyCode ? 'out' : 'in';
    const motionBackdrop = backdrop;

    onlyCode = nextOnlyCode;

    if (onlyCode) {
      includeBackground = false;
      includeTitleBar = false;
    } else {
      includeBackground = true;
      includeTitleBar = true;
    }

    cueBackgroundMotion(backgroundDirection, motionBackdrop);
  }

  function onIncludeBackgroundChange(event: Event): void {
    const nextIncludeBackground = (event.currentTarget as HTMLInputElement).checked;
    const motionBackdrop = backdrop;

    includeBackground = nextIncludeBackground;
    cueBackgroundMotion(includeBackground ? 'in' : 'out', motionBackdrop);
  }

  function onIncludeTitleBarChange(event: Event): void {
    includeTitleBar = (event.currentTarget as HTMLInputElement).checked;
  }

  function onMarkdownPreviewChange(event: Event): void {
    renderMarkdownPreview = (event.currentTarget as HTMLInputElement).checked;
    cuePreviewMotion();
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
  <section bind:this={stageNode} class="stage" class:no-background={!includeBackground && !onlyCode} class:only-code={onlyCode} class:aurora={backdrop === 'aurora'} class:graphite={backdrop === 'graphite'} class:paper={backdrop === 'paper'} class:solid={backdrop === 'solid'} aria-label="Editable code snapshot">
    {#key backgroundMotionKey}
      {#if backgroundFlashMode}
        <span class={`stage-flash ${backgroundFlashBackdrop}`} class:out={backgroundFlashMode === 'out'} aria-hidden="true"></span>
      {/if}
    {/key}

    {#key previewMotionKey}
      <article class="code-window" class:only-code={onlyCode} bind:this={previewNode}>
        {#if includeTitleBar && !onlyCode}
          <header class="window-bar" transition:terminalBar>
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
    {/key}
  </section>

  <aside class="controls" aria-label="Snapshot settings">
    <div class="brand-row">
      <strong>CodeSnap</strong>
      {#if actionLabel}
        <span data-state={exportState}>{actionLabel}</span>
      {/if}
    </div>

    <div class="control-grid">
      <label class="field-card">
        <span>Language</span>
        <CustomSelect
          value={languageChoice}
          options={languageSelectOptions}
          ariaLabel="Choose language"
          on:change={(event) => onLanguageChange(event.detail.value)}
        />
        <small>Wrong language? Choose manually here.</small>
      </label>

      <label class="field-card">
        <span>IDE theme</span>
        <CustomSelect
          value={editorTheme}
          options={themes}
          ariaLabel="Choose IDE theme"
          on:change={(event) => onThemeChange(event.detail.value)}
        />
      </label>
    </div>

    <label class="field-card">
      <span>Background</span>
      <CustomSelect
        value={backdrop}
        options={backgrounds}
        ariaLabel="Choose background"
        on:change={(event) => onBackdropChange(event.detail.value as Backdrop)}
      />
    </label>

    <label class="range-label field-card">
      <span>Line height (default 1.55) <strong>{lineHeight.toFixed(2)}</strong></span>
      <input type="range" min="1.2" max="2" step="0.05" bind:value={lineHeight} />
    </label>

    {#if isMarkdown}
      <label class="toggle-row">
        <span>Markdown preview</span>
        <span class="switch">
          <input type="checkbox" checked={renderMarkdownPreview} on:change={onMarkdownPreviewChange} />
          <span class="switch-track" aria-hidden="true">
            <span class="switch-thumb"></span>
          </span>
        </span>
      </label>
    {/if}

    <label class="toggle-row">
      <span>Only code</span>
      <span class="switch">
        <input type="checkbox" checked={onlyCode} on:change={onOnlyCodeChange} />
        <span class="switch-track" aria-hidden="true">
          <span class="switch-thumb"></span>
        </span>
      </span>
    </label>

    <label class="toggle-row" class:disabled={onlyCode}>
      <span>Include background</span>
      <span class="switch">
        <input type="checkbox" checked={includeBackground} on:change={onIncludeBackgroundChange} disabled={onlyCode} />
        <span class="switch-track" aria-hidden="true">
          <span class="switch-thumb"></span>
        </span>
      </span>
    </label>

    <label class="toggle-row" class:disabled={onlyCode}>
      <span>Include title bar</span>
      <span class="switch">
        <input type="checkbox" checked={includeTitleBar} on:change={onIncludeTitleBarChange} disabled={onlyCode} />
        <span class="switch-track" aria-hidden="true">
          <span class="switch-thumb"></span>
        </span>
      </span>
    </label>

    <div class="actions">
      <button class="secondary" type="button" aria-label="Paste clipboard" on:click={loadClipboard}>
        <Clipboard size={18} strokeWidth={2.25} aria-hidden="true" />
      </button>
      <div class="icon-actions">
        <button class="icon-action" type="button" aria-label="Save PNG" disabled={isBusy} on:click={savePng}>
          <Save size={19} strokeWidth={2.25} />
          <span>Save PNG</span>
        </button>
        <button class="icon-action ghost" type="button" aria-label="Copy PNG" disabled={isBusy} on:click={copyPng}>
          <Copy size={19} strokeWidth={2.25} />
          <span>Copy PNG</span>
        </button>
      </div>
    </div>
  </aside>
</main>
