<script lang="ts">
    /// Destructive actions are undoable rather than confirmed: the schema
    /// already soft-deletes, so the cheap thing to offer is the way back.
    let { message, onUndo, onDismiss } = $props<{
        message: string;
        onUndo: () => void;
        onDismiss: () => void;
    }>();

    const TOTAL_MS = 5000;
    let remaining = $state(TOTAL_MS);
    let interval: number | undefined;

    $effect(() => {
        const start = Date.now();
        interval = setInterval(() => {
            remaining = Math.max(0, TOTAL_MS - (Date.now() - start));
            if (remaining <= 0) {
                clearInterval(interval);
                onDismiss();
            }
        }, 100) as unknown as number;
        return () => clearInterval(interval);
    });

    const widthPct = $derived((remaining / TOTAL_MS) * 100);
</script>

<div
    role="status"
    class="dialog relative overflow-hidden flex items-center gap-3 pl-3 pr-2 py-2 max-w-[min(28rem,90vw)] rounded-[10px]"
>
    <span class="text-[13px] text-[color:var(--text)] break-words">{message}</span>
    <button
        class="btn btn-primary shrink-0"
        style="height: 26px"
        onclick={() => {
            clearInterval(interval);
            onUndo();
        }}
    >
        Undo
    </button>
    <span class="text-[11px] t-num text-[color:var(--text-faint)] shrink-0 w-4 text-right">
        {Math.ceil(remaining / 1000)}
    </span>
    <div
        class="absolute bottom-0 left-0 h-0.5 bg-[color:var(--accent)]"
        style="width: {widthPct}%; transition: width 100ms linear"
    ></div>
</div>
