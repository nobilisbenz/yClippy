<script lang="ts">
    let { message, kind = "info", onClose, key } = $props<{
        message: string;
        kind?: "info" | "success" | "error";
        onClose?: () => void;
        key?: number;
    }>();

    let visible = $state(true);
    let timeout: number | undefined;

    $effect(() => {
        timeout = setTimeout(() => {
            visible = false;
            setTimeout(() => onClose?.(), 200);
        }, 3000) as unknown as number;
        return () => clearTimeout(timeout);
    });

    function dismiss() {
        visible = false;
        clearTimeout(timeout);
        onClose?.();
    }

    let bgClass = $derived(
        kind === "success"
            ? "bg-green-900/90 border-green-700"
            : kind === "error"
                ? "bg-red-900/90 border-red-700"
                : "bg-zinc-900/90 border-zinc-700",
    );
</script>

{#if visible}
    <div
        role="status"
        class="fixed bottom-6 left-1/2 -translate-x-1/2 z-[200] {bgClass} text-white px-4 py-3 rounded-lg border shadow-xl flex items-center gap-3 max-w-md backdrop-blur-sm"
        style="transition: opacity 200ms ease;"
    >
        <span class="text-sm">{message}</span>
        <button
            onclick={dismiss}
            class="ml-auto p-1 hover:bg-white/10 rounded"
            aria-label="Dismiss"
        >
            <svg class="size-3" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
            </svg>
        </button>
    </div>
{/if}