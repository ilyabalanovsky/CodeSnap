<script lang="ts">
  import { ChevronLeft, ChevronRight, Clipboard, Code2, Copy, ImageIcon, Keyboard, Palette, Save, Settings as SettingsIcon, SlidersHorizontal, ToggleLeft, X } from '@lucide/svelte';
  import { onMount, tick } from 'svelte';
  import { listen, type UnlistenFn } from '@tauri-apps/api/event';
  import CodeEditor from './lib/CodeEditor.svelte';
  import CustomSelect from './lib/CustomSelect.svelte';
  import MarkdownPreview from './lib/MarkdownPreview.svelte';
  import { detectLanguageFromCode, extensionForLanguage, languageOptions } from './lib/languages';
  import { copySnapshotPng, defaultAppSettings, getAppSettings, getClipboardText, hideToTray, saveSnapshotPng, setAppSettings, type AppSettings } from './lib/tauri';
  import { copyDataUrlToClipboard, renderNodeToPngDataUrl } from './lib/snapshot';

  type Backdrop = 'aurora' | 'red' | 'violet' | 'blue' | 'sunset' | 'mint' | 'graphite' | 'paper' | 'solid';
  type ExportState = 'idle' | 'saving' | 'copying' | 'saved' | 'copied' | 'failed';
  type CapturedCodePayload = {
    code: string;
    source: string;
  };
  type LanguageBadge = {
    label: string;
    className: string;
  };

  const codeCapturedEvent = 'codesnap://code-captured';
  const uiHiddenEvent = 'codesnap://ui-hidden';
  const uiShownEvent = 'codesnap://ui-shown';

  const sampleCode = `type Mood = "focused" | "curious" | "slightly-caffeinated";

type Snapshot = {
  language: "typescript" | "rust" | "python";
  mood: Mood;
  scale: 2;
};

export function createCodeSnap(source: string): Snapshot {
  const mood = source.includes("TODO") ? "curious" : "focused";
  return renderSnapshot(source, {
    theme: "darcula",
    mood,
    snacks: "optional",
  });
}`;

  const themes = [
    { value: 'darcula', label: 'Darcula' },
    { value: 'dracula', label: 'Dracula' },
    { value: 'githubDark', label: 'GitHub Dark' },
    { value: 'githubLight', label: 'GitHub Light' },
    { value: 'materialDark', label: 'Material Dark' },
    { value: 'monokai', label: 'Monokai' },
    { value: 'nord', label: 'Nord' },
    { value: 'oneDark', label: 'One Dark' },
    { value: 'tokyoNight', label: 'Tokyo Night' },
    { value: 'vscodeDark', label: 'VS Code Dark' },
    { value: 'atomone', label: 'Atom One' },
    { value: 'sublime', label: 'Sublime' },
    { value: 'solarizedDark', label: 'Solarized Dark' },
    { value: 'solarizedLight', label: 'Solarized Light' },
    { value: 'xcodeDark', label: 'Xcode Dark' },
    { value: 'xcodeLight', label: 'Xcode Light' },
    { value: 'quietlight', label: 'Quiet Light' },
  ];

  const backgrounds = [
    { value: 'aurora', label: 'Aurora' },
    { value: 'red', label: 'Ruby' },
    { value: 'violet', label: 'Violet' },
    { value: 'blue', label: 'Ocean' },
    { value: 'sunset', label: 'Sunset' },
    { value: 'mint', label: 'Mint' },
    { value: 'graphite', label: 'Graphite' },
    { value: 'paper', label: 'Paper' },
    { value: 'solid', label: 'Solid' },
  ];

  const languageBadges: Record<string, LanguageBadge> = {
    'C': { label: 'C', className: 'c' },
    'C#': { label: 'C#', className: 'csharp' },
    'C++': { label: 'C++', className: 'cpp' },
    'CSS': { label: 'CSS', className: 'css' },
    'Go': { label: 'GO', className: 'go' },
    'HTML': { label: '<>', className: 'html' },
    'Java': { label: 'JV', className: 'java' },
    'JavaScript': { label: 'JS', className: 'javascript' },
    'JSON': { label: '{}', className: 'json' },
    'Markdown': { label: 'MD', className: 'markdown' },
    'PHP': { label: 'PHP', className: 'php' },
    'Plain Text': { label: 'TXT', className: 'text' },
    'Python': { label: 'PY', className: 'python' },
    'Rust': { label: 'RS', className: 'rust' },
    'Shell': { label: '$', className: 'shell' },
    'SQL': { label: 'SQL', className: 'sql' },
    'TypeScript': { label: 'TS', className: 'typescript' },
    'YAML': { label: 'YML', className: 'yaml' },
  };

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
  let includeLineNumbers = true;
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
  let backgroundOffset = 0;
  let appSettings: AppSettings = defaultAppSettings;
  let isSettingsOpen = false;
  let isHotkeyOpen = false;
  let hotkeyDraft = defaultAppSettings.captureHotkey;
  let settingsError = '';
  let isUiSuspended = false;

  $: isBusy = exportState === 'saving' || exportState === 'copying';
  $: detectedLanguage = detectLanguageFromCode(code);
  $: languageSelectOptions = [{ value: '__auto', label: `Auto detect (${detectedLanguage})` }, ...languageOptions];
  $: visibleBackgrounds = backgrounds.slice(backgroundOffset, backgroundOffset + 4);
  $: canMoveBackgroundsLeft = backgroundOffset > 0;
  $: canMoveBackgroundsRight = backgroundOffset < backgrounds.length - 4;
  $: activeLanguage = languageChoice === '__auto' ? detectedLanguage : languageChoice;
  $: isMarkdown = activeLanguage === 'Markdown';
  $: isMarkdownPreview = isMarkdown && renderMarkdownPreview;
  $: previewLineCount = Math.max(1, code.split('\n').length);
  $: fileExtension = extensionForLanguage(activeLanguage);
  $: fileTabBadge = badgeForLanguage(activeLanguage, fileExtension);
  $: if (fileExtension !== lastLanguageExtension) {
    if (fileName === `snippet.${lastLanguageExtension}` || fileName === 'snippet') {
      fileName = `snippet.${fileExtension}`;
    }
    lastLanguageExtension = fileExtension;
  }
  function updateCode(nextCode: string): void {
    code = nextCode;
  }

  function badgeForLanguage(languageName: string, extension: string): LanguageBadge {
    const knownBadge = languageBadges[languageName];

    if (knownBadge) {
      return knownBadge;
    }

    const safeExtension = extension.replace(/[^a-z0-9#+]/gi, '').slice(0, 3).toUpperCase();
    const className = languageName.toLowerCase().replace(/[^a-z0-9]+/g, '-').replace(/^-|-$/g, '') || 'generic';

    return {
      label: safeExtension || 'CODE',
      className: `generic ${className}`,
    };
  }

  function openSettings(): void {
    settingsError = '';
    isSettingsOpen = true;
  }

  function closeSettings(): void {
    isSettingsOpen = false;
    isHotkeyOpen = false;
    settingsError = '';
  }

  function openHotkeySettings(): void {
    hotkeyDraft = appSettings.captureHotkey;
    settingsError = '';
    isHotkeyOpen = true;
  }

  function closeHotkeySettings(): void {
    isHotkeyOpen = false;
    settingsError = '';
  }

  function normalizeKeyName(key: string): string {
    if (key === ' ') {
      return 'Space';
    }

    if (key === 'Escape') {
      return 'Esc';
    }

    if (key.length === 1) {
      return key.toUpperCase();
    }

    return key.replace(/^Arrow/, '');
  }

  function hotkeyFromEvent(event: KeyboardEvent): string {
    const parts: string[] = [];

    if (event.ctrlKey) {
      parts.push('Ctrl');
    }

    if (event.altKey) {
      parts.push('Alt');
    }

    if (event.shiftKey) {
      parts.push('Shift');
    }

    if (event.metaKey) {
      parts.push('Super');
    }

    if (!['Control', 'Alt', 'Shift', 'Meta'].includes(event.key)) {
      parts.push(normalizeKeyName(event.key));
    }

    return parts.join('+');
  }

  function isCompleteHotkey(hotkey: string): boolean {
    const parts = hotkey.split('+').filter(Boolean);
    const hasModifier = parts.some((part) => ['Ctrl', 'Alt', 'Shift', 'Super'].includes(part));

    return hasModifier && parts.length >= 2;
  }

  function onHotkeyCapture(event: KeyboardEvent): void {
    event.preventDefault();
    event.stopPropagation();

    const nextHotkey = hotkeyFromEvent(event);

    if (nextHotkey) {
      hotkeyDraft = nextHotkey;
      settingsError = '';
    }
  }

  async function saveHotkeySettings(): Promise<void> {
    if (!isCompleteHotkey(hotkeyDraft)) {
      settingsError = 'Press a shortcut with at least one modifier.';
      return;
    }

    await updateAppSettings({ captureHotkey: hotkeyDraft });
    isHotkeyOpen = false;
  }

  async function updateAppSettings(patch: Partial<AppSettings>): Promise<void> {
    const nextSettings = { ...appSettings, ...patch };

    settingsError = '';

    try {
      appSettings = await setAppSettings(nextSettings);
    } catch (error) {
      settingsError = error instanceof Error ? error.message : String(error || 'Could not save settings.');
    }
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

  function moveBackgrounds(direction: -1 | 1): void {
    backgroundOffset = Math.min(Math.max(backgroundOffset + direction, 0), backgrounds.length - 4);
  }

  function onBackgroundWheel(event: WheelEvent): void {
    const delta = Math.abs(event.deltaX) > Math.abs(event.deltaY) ? event.deltaX : event.deltaY;

    if (Math.abs(delta) < 2) {
      return;
    }

    event.preventDefault();
    moveBackgrounds(delta > 0 ? 1 : -1);
  }

  function onOnlyCodeChange(event: Event): void {
    const nextOnlyCode = (event.currentTarget as HTMLInputElement).checked;
    const backgroundDirection = nextOnlyCode ? 'out' : 'in';
    const motionBackdrop = backdrop;

    onlyCode = nextOnlyCode;

    if (onlyCode) {
      includeBackground = false;
      includeTitleBar = false;
      includeLineNumbers = false;
    } else {
      includeBackground = true;
      includeTitleBar = true;
      includeLineNumbers = true;
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

  function onIncludeLineNumbersChange(event: Event): void {
    includeLineNumbers = (event.currentTarget as HTMLInputElement).checked;
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
      cuePreviewMotion();
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
      const exportNode = onlyCode || !includeBackground ? previewNode : stageNode;
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

  onMount(() => {
    let unlisten: UnlistenFn | undefined;

    void getAppSettings().then((settings) => {
      appSettings = settings;
      hotkeyDraft = settings.captureHotkey;
      isUiSuspended = settings.startInTray && document.visibilityState === 'hidden';
    });

    const listeners: Promise<UnlistenFn>[] = [];

    listeners.push(listen<CapturedCodePayload>(codeCapturedEvent, (event) => {
      const nextCode = event.payload.code;

      if (!nextCode.trim()) {
        return;
      }

      code = nextCode;
      languageChoice = '__auto';
      cuePreviewMotion();
      isUiSuspended = false;
    }));

    listeners.push(listen(uiHiddenEvent, () => {
      isUiSuspended = true;
    }));

    listeners.push(listen(uiShownEvent, () => {
      isUiSuspended = false;
    }));

    void Promise.all(listeners)
      .then((unlisteners) => {
        unlisten = () => unlisteners.forEach((item) => item());
      })
      .catch(() => {
        // Browser preview mode does not expose Tauri events.
      });

    return () => {
      unlisten?.();
    };
  });
</script>

<svelte:head>
  <title>CodeSnap</title>
</svelte:head>

<main class="app-shell" class:ui-suspended={isUiSuspended} class:motion-disabled={appSettings.disableAnimations}>
  <section bind:this={stageNode} class="stage" class:no-background={!includeBackground && !onlyCode} class:only-code={onlyCode} class:aurora={backdrop === 'aurora'} class:red={backdrop === 'red'} class:violet={backdrop === 'violet'} class:blue={backdrop === 'blue'} class:sunset={backdrop === 'sunset'} class:mint={backdrop === 'mint'} class:graphite={backdrop === 'graphite'} class:paper={backdrop === 'paper'} class:solid={backdrop === 'solid'} aria-label="Editable code snapshot">
    {#key backgroundMotionKey}
      {#if backgroundFlashMode}
        <span class={`stage-flash ${backgroundFlashBackdrop}`} class:out={backgroundFlashMode === 'out'} aria-hidden="true"></span>
      {/if}
    {/key}

    {#key previewMotionKey}
      <article
        class="code-window"
        class:only-code={onlyCode}
        style={`--preview-lines: ${previewLineCount}; --preview-line-height: ${lineHeight};`}
        bind:this={previewNode}
      >
        {#if includeTitleBar && !onlyCode}
          <header class="window-bar" transition:terminalBar>
            <div class="traffic" aria-hidden="true">
              <span></span>
              <span></span>
              <span></span>
            </div>
            <div class="file-tab">
              <span class={`file-tab-icon ${fileTabBadge.className}`} aria-hidden="true">{fileTabBadge.label}</span>
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
            </div>
          </header>
        {/if}

        {#if isMarkdownPreview}
          <MarkdownPreview value={code} {lineHeight} />
        {:else}
          <CodeEditor value={code} language={activeLanguage} theme={editorTheme} {lineHeight} showLineNumbers={includeLineNumbers && !onlyCode} onChange={updateCode} />
        {/if}
      </article>
    {/key}
  </section>

  <aside class="controls" aria-label="Snapshot settings">
    <div class="settings-stack">
      <section class="control-section">
        <div class="section-title title-with-action">
          <span>
            <Code2 size={15} strokeWidth={2.25} aria-hidden="true" />
            Language
          </span>
          <button class="settings-trigger" type="button" aria-label="Open settings" on:click={openSettings}>
            <SettingsIcon size={15} strokeWidth={2.35} aria-hidden="true" />
          </button>
        </div>
        <CustomSelect
          value={languageChoice}
          options={languageSelectOptions}
          ariaLabel="Choose language"
          on:change={(event) => onLanguageChange(event.detail.value)}
        />
        <small>Wrong language? Choose manually here.</small>
      </section>

      <section class="control-section">
        <div class="section-title">
          <Palette size={15} strokeWidth={2.25} aria-hidden="true" />
          <span>Theme</span>
        </div>
        <CustomSelect
          value={editorTheme}
          options={themes}
          ariaLabel="Choose IDE theme"
          on:change={(event) => onThemeChange(event.detail.value)}
        />
      </section>

      <section class="control-section">
        <div class="section-title">
          <ImageIcon size={15} strokeWidth={2.25} aria-hidden="true" />
          <span>Background</span>
        </div>
        <div class="background-picker" on:wheel|nonpassive={onBackgroundWheel}>
          <button class="swatch-arrow" type="button" aria-label="Previous backgrounds" disabled={!canMoveBackgroundsLeft} on:click={() => moveBackgrounds(-1)}>
            <ChevronLeft size={16} strokeWidth={2.4} aria-hidden="true" />
          </button>
          <div class="background-swatches" role="radiogroup" aria-label="Choose background">
            {#each visibleBackgrounds as item}
              <button
                class={`background-swatch ${item.value}`}
                class:active={backdrop === item.value}
                type="button"
                role="radio"
                aria-checked={backdrop === item.value}
                aria-label={item.label}
                on:click={() => onBackdropChange(item.value as Backdrop)}
              >
                <span aria-hidden="true"></span>
              </button>
            {/each}
          </div>
          <button class="swatch-arrow" type="button" aria-label="Next backgrounds" disabled={!canMoveBackgroundsRight} on:click={() => moveBackgrounds(1)}>
            <ChevronRight size={16} strokeWidth={2.4} aria-hidden="true" />
          </button>
        </div>
      </section>

      <section class="control-section range-label">
        <div class="section-title split">
          <span>
            <SlidersHorizontal size={15} strokeWidth={2.25} aria-hidden="true" />
            Line height
          </span>
          <strong>{lineHeight.toFixed(2)}</strong>
        </div>
        <input type="range" min="1.2" max="2" step="0.05" bind:value={lineHeight} />
      </section>

      <div class="settings-divider"></div>

      <section class="control-section toggle-section">
        <div class="section-title">
          <ToggleLeft size={15} strokeWidth={2.25} aria-hidden="true" />
          <span>Window controls</span>
        </div>

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

        <label class="toggle-row" class:disabled={onlyCode}>
          <span>Line numbers</span>
          <span class="switch">
            <input type="checkbox" checked={includeLineNumbers} on:change={onIncludeLineNumbersChange} disabled={onlyCode} />
            <span class="switch-track" aria-hidden="true">
              <span class="switch-thumb"></span>
            </span>
          </span>
        </label>
      </section>
    </div>

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

{#if isSettingsOpen}
  <div class="modal-backdrop" role="presentation" on:click={closeSettings}>
    <div class="settings-modal" role="dialog" aria-modal="true" aria-labelledby="settings-title" tabindex="-1" on:click|stopPropagation on:keydown|stopPropagation>
      <header class="modal-header">
        <div>
          <span class="modal-kicker">CodeSnap</span>
          <h2 id="settings-title">Settings</h2>
        </div>
        <button class="modal-close" type="button" aria-label="Close settings" on:click={closeSettings}>
          <X size={18} strokeWidth={2.35} aria-hidden="true" />
        </button>
      </header>

      <div class="modal-content">
        <div class="setting-line hotkey-line">
          <div>
            <strong>Capture hotkey</strong>
            <span>{appSettings.captureHotkey}</span>
          </div>
          <button class="mini-action" type="button" on:click={openHotkeySettings}>
            <Keyboard size={15} strokeWidth={2.25} aria-hidden="true" />
            Change
          </button>
        </div>

        <label class="setting-line">
          <span>
            <strong>Launch at login</strong>
            <small>Start CodeSnap together with the system.</small>
          </span>
          <span class="switch">
            <input
              type="checkbox"
              checked={appSettings.launchAtLogin}
              on:change={(event) => updateAppSettings({ launchAtLogin: (event.currentTarget as HTMLInputElement).checked })}
            />
            <span class="switch-track" aria-hidden="true">
              <span class="switch-thumb"></span>
            </span>
          </span>
        </label>

        <label class="setting-line">
          <span>
            <strong>Start in tray</strong>
            <small>Keep the window hidden until the hotkey is pressed.</small>
          </span>
          <span class="switch">
            <input
              type="checkbox"
              checked={appSettings.startInTray}
              on:change={(event) => updateAppSettings({ startInTray: (event.currentTarget as HTMLInputElement).checked })}
            />
            <span class="switch-track" aria-hidden="true">
              <span class="switch-thumb"></span>
            </span>
          </span>
        </label>

        <label class="setting-line">
          <span>
            <strong>Disable animations</strong>
            <small>For maximum performance and the quietest background usage.</small>
          </span>
          <span class="switch">
            <input
              type="checkbox"
              checked={appSettings.disableAnimations}
              on:change={(event) => updateAppSettings({ disableAnimations: (event.currentTarget as HTMLInputElement).checked })}
            />
            <span class="switch-track" aria-hidden="true">
              <span class="switch-thumb"></span>
            </span>
          </span>
        </label>

        {#if settingsError}
          <p class="settings-error">{settingsError}</p>
        {/if}
      </div>
    </div>
  </div>
{/if}

{#if isHotkeyOpen}
  <div class="modal-backdrop hotkey-backdrop" role="presentation" on:click={closeHotkeySettings}>
    <div class="settings-modal hotkey-modal" role="dialog" aria-modal="true" aria-labelledby="hotkey-title" tabindex="-1" on:click|stopPropagation on:keydown|stopPropagation>
      <header class="modal-header">
        <div>
          <span class="modal-kicker">Shortcut</span>
          <h2 id="hotkey-title">Press new hotkey</h2>
        </div>
        <button class="modal-close" type="button" aria-label="Close hotkey settings" on:click={closeHotkeySettings}>
          <X size={18} strokeWidth={2.35} aria-hidden="true" />
        </button>
      </header>

      <div class="modal-content">
        <input
          class="hotkey-input"
          aria-label="New capture hotkey"
          readonly
          value={hotkeyDraft}
          on:keydown={onHotkeyCapture}
          on:focus={(event) => (event.currentTarget as HTMLInputElement).select()}
        />
        <small class="hotkey-hint">Use a modifier combo, for example Ctrl+Shift+S.</small>
        {#if settingsError}
          <p class="settings-error">{settingsError}</p>
        {/if}
        <div class="modal-actions">
          <button class="mini-action ghost" type="button" on:click={closeHotkeySettings}>Cancel</button>
          <button class="mini-action" type="button" disabled={!isCompleteHotkey(hotkeyDraft)} on:click={saveHotkeySettings}>Save hotkey</button>
        </div>
      </div>
    </div>
  </div>
{/if}
