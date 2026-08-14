<script lang="ts" module>
    /// One icon set for the whole app.
    ///
    /// The transport used to be text glyphs — ▶ ⏮ ⏭ ‹ ✕ — which a system
    /// emoji font renders in its own colour and its own optical size, so the
    /// controls came out orange and misaligned against a monochrome UI. These
    /// are stroked paths in `currentColor`: one weight, one grid, themable.
    export type IconName =
        | "play"
        | "pause"
        | "back10"
        | "forward10"
        | "markIn"
        | "markOut"
        | "scissors"
        | "folder"
        | "video"
        | "clip"
        | "chevronRight"
        | "chevronLeft"
        | "chevronUp"
        | "chevronDown"
        | "arrowUp"
        | "arrowDown"
        | "plus"
        | "folderPlus"
        | "settings"
        | "sync"
        | "search"
        | "close"
        | "trash"
        | "pencil"
        | "copy"
        | "external"
        | "check"
        | "alert"
        | "reorder"
        | "more"
        | "minimize"
        | "maximize"
        | "speed"
        | "back";

    type Shape = { d: string; fill?: boolean };

    const ICONS: Record<IconName, Shape[]> = {
        play: [{ d: "M7 4.5v15l12-7.5-12-7.5z", fill: true }],
        pause: [{ d: "M8.5 5v14M15.5 5v14" }],
        back10: [
            { d: "M11 8H6.5a3.5 3.5 0 100 7H8" },
            { d: "M9 5.5 6 8l3 2.5" },
            { d: "M14 15V9l-1.5 1M17.5 9.5v5a1 1 0 001 1h.5a1 1 0 001-1v-5a1 1 0 00-1-1H18.5a1 1 0 00-1 1z" },
        ],
        forward10: [
            { d: "M13 8h4.5a3.5 3.5 0 110 7H16" },
            { d: "M15 5.5 18 8l-3 2.5" },
            { d: "M5.5 15V9L4 10M9 9.5v5a1 1 0 001 1h.5a1 1 0 001-1v-5a1 1 0 00-1-1H10a1 1 0 00-1 1z" },
        ],
        markIn: [{ d: "M8 4v16" }, { d: "M8 4h8M8 20h8" }, { d: "M12 12h8m0 0-3-3m3 3-3 3" }],
        markOut: [{ d: "M16 4v16" }, { d: "M8 4h8M8 20h8" }, { d: "M12 12H4m0 0 3-3m-3 3 3 3" }],
        scissors: [
            { d: "M6.5 6.5a2.5 2.5 0 105 0 2.5 2.5 0 00-5 0zM6.5 17.5a2.5 2.5 0 105 0 2.5 2.5 0 00-5 0z" },
            { d: "M10.8 8.3 20 19M10.8 15.7 20 5" },
        ],
        folder: [{ d: "M3 7.5A1.5 1.5 0 014.5 6h4l2 2.5h7A1.5 1.5 0 0119 10v7.5a1.5 1.5 0 01-1.5 1.5h-13A1.5 1.5 0 013 17.5v-10z" }],
        video: [{ d: "M3 7.5A1.5 1.5 0 014.5 6h9A1.5 1.5 0 0115 7.5v9A1.5 1.5 0 0113.5 18h-9A1.5 1.5 0 013 16.5v-9z" }, { d: "M15 10.5l5-2.5v8l-5-2.5" }],
        clip: [{ d: "M12 21a9 9 0 100-18 9 9 0 000 18z" }, { d: "M10.5 9.5v5l4-2.5-4-2.5z", fill: true }],
        chevronRight: [{ d: "m9.5 5.5 6.5 6.5-6.5 6.5" }],
        chevronLeft: [{ d: "m14.5 5.5-6.5 6.5 6.5 6.5" }],
        chevronUp: [{ d: "m5.5 14.5 6.5-6.5 6.5 6.5" }],
        chevronDown: [{ d: "m5.5 9.5 6.5 6.5 6.5-6.5" }],
        arrowUp: [{ d: "M12 19V5m0 0-6 6m6-6 6 6" }],
        arrowDown: [{ d: "M12 5v14m0 0 6-6m-6 6-6-6" }],
        plus: [{ d: "M12 5v14M5 12h14" }],
        folderPlus: [
            { d: "M3 7.5A1.5 1.5 0 014.5 6h4l2 2.5h7A1.5 1.5 0 0119 10v7.5a1.5 1.5 0 01-1.5 1.5h-13A1.5 1.5 0 013 17.5v-10z" },
            { d: "M11 13.5h4M13 11.5v4" },
        ],
        settings: [
            { d: "M12 15a3 3 0 100-6 3 3 0 000 6z" },
            {
                d: "M19.4 14a1.4 1.4 0 00.3 1.5l.1.1a1.7 1.7 0 11-2.4 2.4l-.1-.1a1.4 1.4 0 00-1.5-.3 1.4 1.4 0 00-.9 1.3v.2a1.7 1.7 0 11-3.4 0v-.1a1.4 1.4 0 00-.9-1.3 1.4 1.4 0 00-1.5.3l-.1.1a1.7 1.7 0 11-2.4-2.4l.1-.1a1.4 1.4 0 00.3-1.5 1.4 1.4 0 00-1.3-.9h-.2a1.7 1.7 0 110-3.4h.1a1.4 1.4 0 001.3-.9 1.4 1.4 0 00-.3-1.5l-.1-.1a1.7 1.7 0 112.4-2.4l.1.1a1.4 1.4 0 001.5.3h.1a1.4 1.4 0 00.9-1.3v-.2a1.7 1.7 0 013.4 0v.1a1.4 1.4 0 00.9 1.3 1.4 1.4 0 001.5-.3l.1-.1a1.7 1.7 0 112.4 2.4l-.1.1a1.4 1.4 0 00-.3 1.5v.1a1.4 1.4 0 001.3.9h.2a1.7 1.7 0 010 3.4h-.1a1.4 1.4 0 00-1.3.9z",
            },
        ],
        sync: [{ d: "M20 11a8 8 0 10-1.2 5.2" }, { d: "M20 5v6h-6" }],
        search: [{ d: "M11 18a7 7 0 100-14 7 7 0 000 14zM20 20l-4.2-4.2" }],
        close: [{ d: "M6 6l12 12M18 6L6 18" }],
        trash: [
            { d: "M4 7h16" },
            { d: "M9.5 7V5.5A1.5 1.5 0 0111 4h2a1.5 1.5 0 011.5 1.5V7" },
            { d: "M6 7l.8 11.2A2 2 0 008.8 20h6.4a2 2 0 002-1.8L18 7" },
            { d: "M10.5 11v5M13.5 11v5" },
        ],
        pencil: [{ d: "M15.2 5.2 18.8 8.8M4 20l.7-3.5L16 5.2a1.7 1.7 0 012.4 0l1.4 1.4a1.7 1.7 0 010 2.4L8.5 19.3 5 20z" }],
        copy: [
            { d: "M9 9.5A1.5 1.5 0 0110.5 8h7A1.5 1.5 0 0119 9.5v7a1.5 1.5 0 01-1.5 1.5h-7A1.5 1.5 0 019 16.5v-7z" },
            { d: "M15 8V6.5A1.5 1.5 0 0013.5 5h-7A1.5 1.5 0 005 6.5v7A1.5 1.5 0 006.5 15H8" },
        ],
        external: [{ d: "M13 5h6v6" }, { d: "M19 5l-8 8" }, { d: "M18 14v4.5A1.5 1.5 0 0116.5 20h-11A1.5 1.5 0 014 18.5v-11A1.5 1.5 0 015.5 6H10" }],
        check: [{ d: "M5 12.5l4.5 4.5L19 7.5" }],
        alert: [{ d: "M12 8.5v4.5M12 16.2v.3" }, { d: "M12 3.5 21 19.5H3L12 3.5z" }],
        reorder: [{ d: "M4 7h16M4 12h16M4 17h16" }],
        more: [
            {
                d: "M12 7.5a1.4 1.4 0 100-2.8 1.4 1.4 0 000 2.8zm0 5.9a1.4 1.4 0 100-2.8 1.4 1.4 0 000 2.8zm0 5.9a1.4 1.4 0 100-2.8 1.4 1.4 0 000 2.8z",
                fill: true,
            },
        ],
        minimize: [{ d: "M6 12h12" }],
        maximize: [{ d: "M6.5 6.5h11v11h-11z" }],
        speed: [{ d: "M12 20a8 8 0 100-16 8 8 0 000 16z" }, { d: "M12 12l4-3" }],
        back: [{ d: "M14.5 5.5l-6.5 6.5 6.5 6.5" }],
    };

    export const iconNames = Object.keys(ICONS) as IconName[];
    export { ICONS };
</script>

<script lang="ts">
    let {
        name,
        size = 16,
        stroke = 1.75,
        class: className = "",
    }: {
        name: IconName;
        size?: number;
        stroke?: number;
        class?: string;
    } = $props();

    const shapes = $derived(ICONS[name] ?? []);
</script>

<svg
    class={className}
    width={size}
    height={size}
    viewBox="0 0 24 24"
    fill="none"
    stroke="currentColor"
    stroke-width={stroke}
    stroke-linecap="round"
    stroke-linejoin="round"
    aria-hidden="true"
    focusable="false"
>
    {#each shapes as shape}
        <path d={shape.d} fill={shape.fill ? "currentColor" : "none"} stroke={shape.fill ? "none" : "currentColor"} />
    {/each}
</svg>
