<script lang="ts">
    import type { Snippet } from "svelte";
    import Icon from "./Icon.svelte";

    /// One dialog shell: scrim, escape, click-outside, a header that is the
    /// same height as every other panel header, and a footer that is the only
    /// place actions live. Every modal used to redeclare all of that, which is
    /// why no two of them closed the same way.
    let {
        title,
        onClose,
        size = "md",
        children,
        footer,
    }: {
        title: string;
        onClose: () => void;
        size?: "sm" | "md" | "lg";
        children: Snippet;
        footer?: Snippet;
    } = $props();

    const WIDTHS = { sm: "24rem", md: "30rem", lg: "40rem" };
</script>

<svelte:window on:keydown={(e) => e.key === "Escape" && onClose()} />

<div class="overlay z-[220] items-center justify-center p-4" role="presentation" onclick={onClose}>
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
    <div
        class="dialog w-full flex flex-col max-h-[85dvh] overflow-hidden"
        style="max-width: {WIDTHS[size]}"
        role="dialog"
        tabindex="-1"
        aria-modal="true"
        aria-label={title}
        onclick={(e) => e.stopPropagation()}
    >
        <header class="panel-head">
            <h2 class="flex-1 min-w-0 truncate text-[13px] font-semibold text-[color:var(--text)]">
                {title}
            </h2>
            <button class="icon-btn" onclick={onClose} aria-label="Close">
                <Icon name="close" size={15} />
            </button>
        </header>

        <div class="flex-1 min-h-0 overflow-y-auto scroll-thin p-4">
            {@render children()}
        </div>

        {#if footer}
            <footer
                class="shrink-0 flex items-center justify-end gap-2 p-3 border-t border-[color:var(--border)]"
            >
                {@render footer()}
            </footer>
        {/if}
    </div>
</div>
