<script lang="ts">
    import { onMount, tick } from "svelte";

    type MenuItem = {
        label: string;
        action: () => void;
        danger?: boolean;
    };

    let {
        items = [],
        x,
        y,
        onClose,
    } = $props<{
        items: MenuItem[];
        x: number;
        y: number;
        onClose: () => void;
    }>();

    let menuEl: HTMLElement;

    onMount(() => {
        // Adjust position if out of bounds
        (async () => {
            await tick();
            if (menuEl) {
                const rect = menuEl.getBoundingClientRect();
                if (rect.right > window.innerWidth) {
                    menuEl.style.left = `${window.innerWidth - rect.width - 5}px`;
                } else {
                    menuEl.style.left = `${x}px`;
                }

                if (rect.bottom > window.innerHeight) {
                    menuEl.style.top = `${y - rect.height}px`;
                } else {
                    menuEl.style.top = `${y}px`;
                }
            }
        })();

        // Global click listener to close
        const handleClick = () => onClose();
        window.addEventListener("mousedown", handleClick);
        window.addEventListener("dragstart", handleClick);
        window.addEventListener("scroll", handleClick, true); // Capture scroll events
        return () => {
            window.removeEventListener("mousedown", handleClick);
            window.removeEventListener("dragstart", handleClick);
            window.removeEventListener("scroll", handleClick, true);
        };
    });
</script>

<div
    bind:this={menuEl}
    class="fixed z-[280] min-w-[180px] py-1 dialog rounded-[8px] flex flex-col"
    style="left: {x}px; top: {y}px;"
    onmousedown={(e) => e.stopPropagation()}
    onclick={(e) => e.stopPropagation()}
    onkeydown={(e) => e.key === "Escape" && onClose()}
    role="menu"
    tabindex="0"
>
    {#each items as item}
        <button
            class="px-3 py-2 text-left text-[13px] transition-colors flex items-center gap-2
            hover:bg-[color:var(--surface-hi)]
            {item.danger ? 'text-[color:var(--danger)]' : 'text-[color:var(--text-dim)] hover:text-[color:var(--text)]'}"
            onclick={() => {
                item.action();
                onClose();
            }}
        >
            {item.label}
        </button>
    {/each}
</div>
