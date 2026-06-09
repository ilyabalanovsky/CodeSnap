<script lang="ts">
  import { Check, ChevronDown } from '@lucide/svelte';
  import { createEventDispatcher, onMount, tick } from 'svelte';

  export type SelectOption = {
    value: string;
    label: string;
    excludeFromTypeahead?: boolean;
  };

  export let value: string;
  export let options: SelectOption[] = [];
  export let ariaLabel = 'Choose option';

  const dispatch = createEventDispatcher<{ change: { value: string } }>();

  let isOpen = false;
  let activeIndex = 0;
  let rootNode: HTMLDivElement;
  let buttonNode: HTMLButtonElement;
  let menuNode: HTMLDivElement;
  let menuStyle = '';
  let typeahead = '';
  let typeaheadTimer: number | undefined;

  $: selectedIndex = Math.max(
    0,
    options.findIndex((option) => option.value === value),
  );
  $: selectedOption = options[selectedIndex] ?? options[0];

  onMount(() => {
    function onDocumentPointerDown(event: PointerEvent): void {
      const target = event.target as Node;

      if (!rootNode.contains(target) && !menuNode?.contains(target)) {
        isOpen = false;
      }
    }

    function onViewportChange(): void {
      if (isOpen) {
        updateMenuPosition();
      }
    }

    document.addEventListener('pointerdown', onDocumentPointerDown);
    window.addEventListener('resize', onViewportChange);
    window.addEventListener('scroll', onViewportChange, true);

    return () => {
      document.removeEventListener('pointerdown', onDocumentPointerDown);
      window.removeEventListener('resize', onViewportChange);
      window.removeEventListener('scroll', onViewportChange, true);
    };
  });

  async function openMenu(): Promise<void> {
    updateMenuPosition();
    isOpen = true;
    activeIndex = selectedIndex;
    await tick();
    updateMenuPosition();
    scrollActiveOptionIntoView();
  }

  function closeMenu(): void {
    isOpen = false;
  }

  function toggleMenu(): void {
    if (isOpen) {
      closeMenu();
      return;
    }

    void openMenu();
  }

  function chooseOption(nextValue: string): void {
    value = nextValue;
    dispatch('change', { value: nextValue });
    closeMenu();
    buttonNode.focus();
  }

  function moveActive(step: number): void {
    activeIndex = (activeIndex + step + options.length) % options.length;
    void tick().then(scrollActiveOptionIntoView);
  }

  function updateMenuPosition(): void {
    const rect = buttonNode.getBoundingClientRect();
    const viewportPadding = 10;
    const top = Math.min(rect.bottom + 7, window.innerHeight - viewportPadding);
    const left = Math.min(rect.left, window.innerWidth - rect.width - viewportPadding);

    menuStyle = `
      --select-left: ${Math.max(viewportPadding, left)}px;
      --select-top: ${top}px;
      --select-width: ${rect.width}px;
      --select-max-height: ${Math.max(120, window.innerHeight - top - viewportPadding)}px;
    `;
  }

  function scrollActiveOptionIntoView(): void {
    menuNode?.querySelector('[data-active="true"]')?.scrollIntoView({
      block: 'nearest',
    });
  }

  function portal(node: HTMLElement): { destroy: () => void } {
    document.body.appendChild(node);

    return {
      destroy() {
        node.remove();
      },
    };
  }

  function jumpToTypedOption(letter: string): void {
    if (typeaheadTimer) {
      window.clearTimeout(typeaheadTimer);
    }

    typeahead = `${typeahead}${letter.toLowerCase()}`;
    typeaheadTimer = window.setTimeout(() => {
      typeahead = '';
    }, 620);

    const nextIndex = options.findIndex((option) => !option.excludeFromTypeahead && option.label.toLowerCase().startsWith(typeahead));

    if (nextIndex >= 0) {
      activeIndex = nextIndex;
      if (!isOpen) {
        isOpen = true;
      }
      void tick().then(scrollActiveOptionIntoView);
    }
  }

  function onKeydown(event: KeyboardEvent): void {
    if (event.key === 'ArrowDown') {
      event.preventDefault();
      if (!isOpen) {
        void openMenu();
      } else {
        moveActive(1);
      }
      return;
    }

    if (event.key === 'ArrowUp') {
      event.preventDefault();
      if (!isOpen) {
        void openMenu();
      } else {
        moveActive(-1);
      }
      return;
    }

    if (event.key === 'Home') {
      event.preventDefault();
      activeIndex = 0;
      void tick().then(scrollActiveOptionIntoView);
      return;
    }

    if (event.key === 'End') {
      event.preventDefault();
      activeIndex = options.length - 1;
      void tick().then(scrollActiveOptionIntoView);
      return;
    }

    if (event.key === 'Enter' || event.key === ' ') {
      event.preventDefault();
      if (!isOpen) {
        void openMenu();
      } else {
        chooseOption(options[activeIndex]?.value ?? value);
      }
      return;
    }

    if (event.key === 'Escape') {
      event.preventDefault();
      closeMenu();
      buttonNode.focus();
      return;
    }

    if (event.key.length === 1 && !event.ctrlKey && !event.metaKey && !event.altKey) {
      jumpToTypedOption(event.key);
    }
  }
</script>

<div class="custom-select" class:open={isOpen} bind:this={rootNode}>
  <button
    bind:this={buttonNode}
    class="custom-select-button"
    type="button"
    aria-label={ariaLabel}
    aria-haspopup="listbox"
    aria-expanded={isOpen}
    on:click={toggleMenu}
    on:keydown={onKeydown}
  >
    <span>{selectedOption?.label ?? 'Choose'}</span>
    <ChevronDown size={17} strokeWidth={2.25} aria-hidden="true" />
  </button>

  {#if isOpen}
    <div
      bind:this={menuNode}
      use:portal
      class="custom-select-menu"
      role="listbox"
      aria-label={ariaLabel}
      style={menuStyle}
    >
      {#each options as option, index}
        <button
          class="custom-select-option"
          class:active={index === activeIndex}
          class:selected={option.value === value}
          type="button"
          role="option"
          aria-selected={option.value === value}
          data-active={index === activeIndex}
          on:mouseenter={() => (activeIndex = index)}
          on:click={() => chooseOption(option.value)}
        >
          <span>{option.label}</span>
          {#if option.value === value}
            <Check size={15} strokeWidth={2.4} aria-hidden="true" />
          {/if}
        </button>
      {/each}
    </div>
  {/if}
</div>
