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
        class="overlay z-[300] items-center justify-center p-4"
        onclick={onCancel}
        onkeydown={handleKeyDown}
        role="presentation"
    >
        <div
            class="dialog w-full max-w-md overflow-hidden"
            onclick={(e) => e.stopPropagation()}
            onkeydown={handleKeyDown}
            role="dialog"
            aria-modal="true"
            aria-label={title}
            tabindex="-1"
        >
            <header class="panel-head">
                <h3 class="text-[13px] font-semibold text-[color:var(--text)]">{title}</h3>
            </header>
            <div class="p-4">
                {#if message}
                    <p class="text-sm text-[color:var(--text-dim)] leading-relaxed">{message}</p>
                {/if}
                {#if withInput}
                    <input
                        bind:this={inputEl}
                        bind:value={inputValue}
                        type="text"
                        placeholder={promptPlaceholder}
                        class="field mt-3"
                    />
                {/if}
            </div>
            <div class="p-3 border-t border-[color:var(--border)] flex justify-end gap-2">
                <button onclick={onCancel} class="btn btn-ghost">{cancelLabel}</button>
                <button
                    onclick={() => onConfirm(withInput ? inputValue : undefined)}
                    class="btn btn-primary"
                    style={danger
                        ? "background: var(--danger); border-color: var(--danger)"
                        : ""}
                >
                    {confirmLabel}
                </button>
            </div>
        </div>
    </div>
{/if}
