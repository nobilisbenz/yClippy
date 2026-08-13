<script lang="ts">
    let { message, onUndo, onDismiss } = $props<{
        message: string;
        onUndo: () => void;
        onDismiss: () => void;
    }>();

    let remaining = $state(5);
    let interval: number | undefined;

    $effect(() => {
        const start = Date.now();
        const total = 5000;
        interval = setInterval(() => {
            remaining = Math.max(0, Math.ceil((total - (Date.now() - start)) / 1000));
            if (remaining <= 0) {
                clearInterval(interval);
                onDismiss();
            }
        }, 250) as unknown as number;
        return () => clearInterval(interval);
    });

    let widthPct = $derived((remaining / 5) * 100);
</script>

<div
    role="status"
    class="fixed bottom-6 left-1/2 -translate-x-1/2 z-[200] bg-zinc-900/95 text-white px-4 py-3 rounded-lg border border-zinc-700 shadow-xl flex items-center gap-3 max-w-md backdrop-blur-sm"
>
    <span class="text-sm">{message}</span>
    <button
        onclick={() => {
            clearInterval(interval);
            onUndo();
        }}
        class="px-3 py-1 bg-blue-600 hover:bg-blue-500 rounded text-xs font-medium transition-colors"
    >
        Undo
    </button>
    <span class="text-xs text-zinc-500 t-num">{remaining}s</span>
    <div class="absolute bottom-0 left-0 h-0.5 bg-blue-500 transition-all" style="width: {widthPct}%"></div>
</div>