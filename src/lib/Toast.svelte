<script lang="ts">
    import Icon from "./Icon.svelte";

    /// Positioned by the stack in App.svelte, not by itself: two notices that
    /// each claimed the bottom centre used to sit on top of each other.
    let { message, kind = "info", onClose, key } = $props<{
        message: string;
        kind?: "info" | "success" | "error";
        onClose?: () => void;
        key?: number;
    }>();

    let visible = $state(true);
    let timeout: number | undefined;

    $effect(() => {
        key;
        visible = true;
        timeout = setTimeout(() => {
            visible = false;
            setTimeout(() => onClose?.(), 200);
        }, 3500) as unknown as number;
        return () => clearTimeout(timeout);
    });

    function dismiss() {
        visible = false;
        clearTimeout(timeout);
        onClose?.();
    }

    const tint = $derived(
        kind === "success"
            ? "var(--success)"
            : kind === "error"
              ? "var(--danger)"
              : "var(--text-faint)",
    );
</script>

{#if visible}
    <div
        role="status"
        class="dialog flex items-center gap-3 pl-3 pr-1.5 py-2 max-w-[min(28rem,90vw)] rounded-[10px]"
        style="transition: opacity 200ms ease"
    >
        <span style="color: {tint}" class="shrink-0">
            <Icon name={kind === "error" ? "alert" : kind === "success" ? "check" : "clip"} size={15} />
        </span>
        <span class="text-[13px] text-[color:var(--text)] break-words">{message}</span>
        <button class="icon-btn shrink-0" onclick={dismiss} aria-label="Dismiss">
            <Icon name="close" size={14} />
        </button>
    </div>
{/if}
