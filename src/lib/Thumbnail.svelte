<script lang="ts">
    let { videoId, alt = "", className = "" } = $props<{
        videoId: string;
        alt?: string;
        className?: string;
    }>();

    const FALLBACKS = [
        "maxresdefault.jpg",
        "sddefault.jpg",
        "hqdefault.jpg",
        "mqdefault.jpg",
        "default.jpg",
    ];

    let level = $state(0);
    let failed = $state(false);

    let currentSrc = $derived(
        failed ? "" : `https://img.youtube.com/vi/${videoId}/${FALLBACKS[level]}`,
    );

    function handleError() {
        if (level < FALLBACKS.length - 1) {
            level++;
        } else {
            failed = true;
        }
    }
</script>

{#if failed}
    <div
        class="flex items-center justify-center bg-[color:var(--surface-hi)] text-[color:var(--text-faint)] text-xs {className}"
        aria-label="No thumbnail"
    >
        <svg class="size-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
            <path
                stroke-linecap="round"
                stroke-linejoin="round"
                stroke-width="2"
                d="M15 10l4.553-2.276A1 1 0 0121 8.618v6.764a1 1 0 01-1.447.894L15 14M5 18h8a2 2 0 002-2V8a2 2 0 00-2-2H5a2 2 0 00-2 2v8a2 2 0 002 2z"
            />
        </svg>
    </div>
{:else}
    <img
        src={currentSrc}
        alt={alt}
        onerror={handleError}
        class={className}
    />
{/if}