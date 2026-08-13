<script lang="ts">
    let {
        open = false,
        title = "",
        message = "",
        confirmLabel = "Confirm",
        cancelLabel = "Cancel",
        danger = false,
        promptValue = "",
        promptPlaceholder = "",
        withInput = false,
        onConfirm,
        onCancel,
    } = $props<{
        open: boolean;
        title?: string;
        message?: string;
        confirmLabel?: string;
        cancelLabel?: string;
        danger?: boolean;
        promptValue?: string;
        promptPlaceholder?: string;
        withInput?: boolean;
        onConfirm: (value?: string) => void;
        onCancel: () => void;
    }>();

    let inputEl: HTMLInputElement | undefined = $state();
    let inputValue = $state("");

    $effect(() => {
        if (open) {
            inputValue = promptValue;
            queueMicrotask(() => inputEl?.focus());
        }
    });

    function handleKeyDown(e: KeyboardEvent) {
        if (e.key === "Escape") {
            e.preventDefault();
            onCancel();
        } else if (e.key === "Enter" && withInput) {
            e.preventDefault();
            onConfirm(inputValue);
        }
    }
</script>

{#if open}
    <div
        class="fixed inset-0 z-[300] bg-black/70 backdrop-blur-sm flex items-center justify-center p-4"
        onclick={onCancel}
        onkeydown={handleKeyDown}
        role="presentation"
    >
        <div
            class="w-full max-w-md bg-[color:var(--surface)] border border-[color:var(--border)] rounded-xl shadow-2xl overflow-hidden"
            onclick={(e) => e.stopPropagation()}
            onkeydown={handleKeyDown}
            role="dialog"
            aria-modal="true"
            aria-label={title}
            tabindex="-1"
        >
            <div class="p-4 border-b border-[color:var(--border)]">
                <h3 class="text-lg font-bold text-white">{title}</h3>
            </div>
            <div class="p-4">
                {#if message}
                    <p class="text-sm text-zinc-300 leading-relaxed">{message}</p>
                {/if}
                {#if withInput}
                    <input
                        bind:this={inputEl}
                        bind:value={inputValue}
                        type="text"
                        placeholder={promptPlaceholder}
                        class="mt-3 w-full bg-zinc-950 border border-[color:var(--border)] rounded-lg px-3 py-2 text-white focus:outline-none focus:border-[color:var(--accent)] transition text-sm"
                    />
                {/if}
            </div>
            <div class="p-3 border-t border-[color:var(--border)] flex justify-end gap-2">
                <button
                    onclick={onCancel}
                    class="px-4 py-2 bg-zinc-800 hover:bg-zinc-700 text-white rounded text-sm transition-colors"
                >
                    {cancelLabel}
                </button>
                <button
                    onclick={() => onConfirm(withInput ? inputValue : undefined)}
                    class="px-4 py-2 rounded text-white text-sm transition-colors {danger ? 'bg-red-600 hover:bg-red-500' : 'bg-blue-600 hover:bg-blue-500'}"
                >
                    {confirmLabel}
                </button>
            </div>
        </div>
    </div>
{/if}
