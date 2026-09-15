<script lang="ts">
  import { onMount, type Snippet } from 'svelte';
  import { X } from 'lucide-svelte';

  let {
    title,
    variant = 'standard',
    onclose,
    children,
  }: {
    title: string;
    variant?: 'standard' | 'palette';
    onclose: () => void;
    children: Snippet;
  } = $props();

  let dialogEl: HTMLDialogElement;

  onMount(() => {
    const previousActive = document.activeElement;
    dialogEl.showModal();
    const firstInput = dialogEl.querySelector<HTMLInputElement | HTMLButtonElement | HTMLSelectElement>(
      'input, select, button.btn-primary'
    );
    firstInput?.focus();

    return () => {
      if (previousActive instanceof HTMLElement) {
        previousActive.focus();
      }
    };
  });
</script>

<dialog
  bind:this={dialogEl}
  class="nicle-dialog"
  class:wide-palette={variant === 'palette'}
  oncancel={e => {
    e.preventDefault();
    onclose();
  }}
  aria-label={title}
>
  <div class="dialog-header">
    <h2 class="dialog-title">{title}</h2>
    <button class="icon-btn" onclick={onclose} aria-label="Close dialog" title="Close dialog">
      <X size={16} />
    </button>
  </div>
  <div class="dialog-body">
    {@render children()}
  </div>
</dialog>
