<script lang="ts">
    import PopoverShell from "./PopoverShell.svelte";
    import IconCheck from "@tabler/icons-svelte/icons/check";
    import IconX from "@tabler/icons-svelte/icons/x";
    import { cn } from "$lib/utils";

    interface Props {
        value: any;
        mode: "date" | "datetime";
        anchorEl?: HTMLElement | null;
        onCommit: (newValue: any) => void;
        onCancel: () => void;
    }

    let { value, mode, anchorEl, onCommit, onCancel }: Props = $props();

    const isMac =
        typeof navigator !== "undefined" && navigator.userAgent.includes("Mac");

    const monthNames = [
        "January",
        "February",
        "March",
        "April",
        "May",
        "June",
        "July",
        "August",
        "September",
        "October",
        "November",
        "December",
    ];

    function daysInMonth(y: number, m: number) {
        if (!Number.isFinite(y) || !Number.isFinite(m)) return 31;
        return new Date(y, m + 1, 0).getDate();
    }

    function parseIncoming(val: any) {
        const now = new Date();
        const fallback = {
            day: now.getDate(),
            month: now.getMonth(),
            year: now.getFullYear(),
            hour: 0,
            minute: 0,
            second: 0,
        };
        if (!val) return fallback;

        if (typeof val === "string") {
            const m =
                /^(\d{4})-(\d{2})-(\d{2})(?:[T\s](\d{2}):(\d{2})(?::(\d{2}))?(?:Z|[+-]\d{2}:?\d{2})?)?$/.exec(
                    val,
                );
            if (m) {
                const [, y, mo, d, h, mi, s] = m;
                return {
                    year: Number(y),
                    month: Number(mo) - 1,
                    day: Number(d),
                    hour: h !== undefined ? Number(h) : 0,
                    minute: mi !== undefined ? Number(mi) : 0,
                    second: s !== undefined ? Number(s) : 0,
                };
            }
        }

        const base = val instanceof Date ? val : new Date(val);
        if (isNaN(base.getTime())) return fallback;

        return {
            day: base.getDate(),
            month: base.getMonth(),
            year: base.getFullYear(),
            hour: base.getHours(),
            minute: base.getMinutes(),
            second: base.getSeconds(),
        };
    }

    let day = $state<number>(1);
    let month = $state<number>(0);
    let year = $state<number>(new Date().getFullYear());
    let hour = $state<number>(0);
    let minute = $state<number>(0);
    let second = $state<number>(0);

    let dayOptions = $state<number[]>(
        Array.from({ length: 31 }, (_, i) => i + 1),
    );

    // One-time sync from value
    let initialSync = false;
    $effect(() => {
        if (!initialSync) {
            const parsed = parseIncoming(value);
            day = parsed.day;
            month = parsed.month;
            year = parsed.year;
            hour = parsed.hour;
            minute = parsed.minute;
            second = parsed.second;
            initialSync = true;
        }
    });

    $effect(() => {
        const y = Number.isFinite(year)
            ? (year as number)
            : new Date().getFullYear();
        const m = Number.isFinite(month) ? (month as number) : 0;
        const max = daysInMonth(y, m);

        // Correct day if it exceeds max for the month
        if (Number.isFinite(day) && (day as number) > max) {
            day = max;
        }

        // Update day options array only when max changes
        const newOptions = Array.from({ length: max }, (_, i) => i + 1);
        if (dayOptions.length !== newOptions.length) {
            dayOptions = newOptions;
        }
    });

    function handleKeydown(e: KeyboardEvent) {
        if (e.key === "Enter" && (e.metaKey || e.ctrlKey || mode === "date")) {
            e.preventDefault();
            commit();
        }
    }

    function pad(n: number) {
        return n.toString().padStart(2, "0");
    }

    function commit() {
        if (!year || (!month && month !== 0) || !day) {
            onCommit(null);
            return;
        }
        const useHour =
            mode === "datetime"
                ? Number.isFinite(hour)
                    ? (hour as number)
                    : 0
                : 0;
        const useMinute =
            mode === "datetime"
                ? Number.isFinite(minute)
                    ? (minute as number)
                    : 0
                : 0;
        const useSecond =
            mode === "datetime"
                ? Number.isFinite(second)
                    ? (second as number)
                    : 0
                : 0;

        const y = year as number;
        const m = (month as number) + 1;
        const d = day as number;

        if (mode === "date") {
            onCommit(`${y}-${pad(m)}-${pad(d)}`);
            return;
        }

        onCommit(
            `${y}-${pad(m)}-${pad(d)}T${pad(useHour)}:${pad(useMinute)}:${pad(useSecond)}`,
        );
    }
</script>

<PopoverShell {anchorEl} {onCancel} minWidth={280} maxWidth={340}>
    <div class="flex flex-col gap-3 p-3" onkeydown={handleKeydown}>
        <div class="grid grid-cols-3 gap-2">
            <div class="flex flex-col gap-1 text-xs text-muted-foreground">
                <span>Day</span>
                <select
                    class="w-full rounded border border-border px-2 py-1 text-sm bg-background text-foreground focus:outline-none focus:ring-1 focus:ring-border-focus"
                    bind:value={day}
                >
                    {#each dayOptions as d}
                        <option value={d}>{d}</option>
                    {/each}
                </select>
            </div>
            <div class="flex flex-col gap-1 text-xs text-muted-foreground">
                <span>Month</span>
                <select
                    class="w-full rounded border border-border px-2 py-1 text-sm bg-background text-foreground focus:outline-none focus:ring-1 focus:ring-border-focus"
                    bind:value={month}
                >
                    {#each monthNames as label, idx}
                        <option value={idx}>{label}</option>
                    {/each}
                </select>
            </div>
            <div class="flex flex-col gap-1 text-xs text-muted-foreground">
                <span>Year</span>
                <input
                    type="number"
                    class="w-full rounded border border-border px-2 py-1 text-sm bg-background text-foreground focus:outline-none focus:ring-1 focus:ring-border-focus"
                    bind:value={year}
                    min="1900"
                    max="2100"
                    step="1"
                />
            </div>
        </div>

        {#if mode === "datetime"}
            <div class="grid grid-cols-3 gap-2">
                <div class="flex flex-col gap-1 text-xs text-muted-foreground">
                    <span>Hour</span>
                    <input
                        type="number"
                        class="w-full rounded border px-2 py-1 text-sm bg-background"
                        bind:value={hour}
                        min="0"
                        max="23"
                        step="1"
                    />
                </div>
                <div class="flex flex-col gap-1 text-xs text-muted-foreground">
                    <span>Minute</span>
                    <input
                        type="number"
                        class="w-full rounded border px-2 py-1 text-sm bg-background"
                        bind:value={minute}
                        min="0"
                        max="59"
                        step="1"
                    />
                </div>
                <div class="flex flex-col gap-1 text-xs text-muted-foreground">
                    <span>Second</span>
                    <input
                        type="number"
                        class="w-full rounded border px-2 py-1 text-sm bg-background"
                        bind:value={second}
                        min="0"
                        max="59"
                        step="1"
                    />
                </div>
            </div>
        {/if}

        <div class="flex gap-2 text-xs text-muted-foreground">
            <button
                type="button"
                class="rounded border border-accent/10 px-2 py-0.5 text-foreground hover:bg-muted transition text-[11px]"
                onclick={() => {
                    const now = new Date();
                    day = now.getUTCDate();
                    month = now.getUTCMonth();
                    year = now.getUTCFullYear();
                    if (mode === "datetime") {
                        hour = now.getUTCHours();
                        minute = now.getUTCMinutes();
                        second = now.getUTCSeconds();
                    }
                }}
            >
                {mode === "datetime" ? "Now" : "Today"}
            </button>
            <button
                type="button"
                class="rounded border border-accent/10 px-2 py-0.5 hover:bg-muted transition text-[11px]"
                onclick={() => {
                    const parsed = parseIncoming(value);
                    day = parsed.day;
                    month = parsed.month;
                    year = parsed.year;
                    hour = parsed.hour;
                    minute = parsed.minute;
                    second = parsed.second;
                }}
            >
                Reset
            </button>
            <button
                type="button"
                class="rounded border border-accent/10 px-2 py-0.5 hover:bg-muted transition text-[11px]"
                onclick={() => {
                    day = 1;
                    month = 0;
                    year = new Date().getFullYear();
                    hour = 0;
                    minute = 0;
                    second = 0;
                }}
            >
                Clear
            </button>
        </div>

        <div class="flex items-center justify-center gap-2 mt-1">
            <button
                type="button"
                class="flex items-center gap-1.5 px-2 py-0.5 rounded border border-transparent hover:border-accent/10 hover:bg-muted text-foreground-muted transition-colors active:scale-95 group/btn"
                onclick={onCancel}
            >
                <span
                    class="text-[9px] font-medium px-1 rounded bg-black/5 dark:bg-white/5 border border-black/5 dark:border-white/5 text-foreground-muted/60"
                    >Esc</span
                >
                <IconX
                    class="size-3.5 opacity-60 group-hover/btn:opacity-100"
                />
            </button>

            <button
                type="button"
                class="flex items-center gap-1.5 px-2 py-0.5 rounded text-accent border border-transparent hover:border-accent/10 hover:bg-accent/10 transition-colors active:scale-95 group/btn"
                onclick={commit}
            >
                <span
                    class="text-[9px] font-medium px-1 rounded bg-accent/10 border border-accent/20 text-accent/80"
                    >{isMac || mode === "date" ? "↵" : "Ctrl↵"}</span
                >
                <IconCheck
                    class="size-3.5 opacity-80 group-hover/btn:opacity-100"
                />
            </button>
        </div>
    </div>
</PopoverShell>
