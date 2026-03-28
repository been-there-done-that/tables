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
            fractional: "",
            timezone: "",
        };
        if (!val) return fallback;

        if (typeof val === "string") {
            // Capture timezone: Z or +HH:MM or -HH:MM, allowing optional space before it
            // Also supports fractional seconds: .123456
            const m =
                /^(\d{4})-(\d{2})-(\d{2})(?:[T\s](\d{2}):(\d{2})(?::(\d{2})(?:\.(\d+))?)?)?(?:\s*(Z|[+-]\d{2}(?::?\d{2})?))?$/.exec(
                    val,
                );
            if (m) {
                const [, y, mo, d, h, mi, s, frac, tz] = m;
                return {
                    year: Number(y),
                    month: Number(mo) - 1,
                    day: Number(d),
                    hour: h !== undefined ? Number(h) : 0,
                    minute: mi !== undefined ? Number(mi) : 0,
                    second: s !== undefined ? Number(s) : 0,
                    fractional: frac || "",
                    timezone: tz || "",
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
            fractional: "",
            timezone: "",
        };
    }

    let day = $state<number>(1);
    let month = $state<number>(0);
    let year = $state<number>(new Date().getFullYear());
    let hour = $state<number>(0);
    let minute = $state<number>(0);
    let second = $state<number>(0);
    let fractional = $state<string>("");
    let timezone = $state<string>("");

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
            fractional = parsed.fractional;
            timezone = parsed.timezone;
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

    // Derived for 1-based month input
    // svelte-ignore state_referenced_locally
    let displayMonth = $state(month + 1);
    $effect(() => {
        // When internal month changes, update display
        if (displayMonth !== month + 1) {
            displayMonth = month + 1;
        }
    });
    $effect(() => {
        // When display changes (user input), update internal
        if (displayMonth && displayMonth !== month + 1) {
            month = Math.max(0, Math.min(11, displayMonth - 1));
        }
    });

    function convertToLocal() {
        // Use the current values to construct a date object
        // If we have a timezone, we construct the date as ISO with that timezone, then read back the local components
        // If we don't have a timezone, we assume it was already parsed as local components or just UTC numbers...
        // Actually, if we have "2023-01-01T12:00:00Z", we parsed it as 12, 00, 00, Z.
        // We want to convert 12:00 Z to Local.

        let dateStr = `${year}-${pad(month + 1)}-${pad(day)}`;
        if (mode === "datetime") {
            dateStr += `T${pad(hour)}:${pad(minute)}:${pad(second)}`;
            if (fractional) {
                dateStr += `.${fractional}`;
            }
        }

        // If we have a timezone, append it. If not, assume UTC if we want to convert TO local?
        // Or if no timezone, assume it IS local/floating and do nothing?
        // Usually "Convert to Local" implies "Treat this as UTC/Server time and show me what it is here".
        // But if it already has a timezone, proceed.
        // If it has NO timezone, let's treat it as UTC for conversion purposes if that's the standard, or just return.

        let d: Date;
        if (timezone) {
            d = new Date(dateStr + timezone);
        } else {
            // If no timezone is present, assume UTC for conversion? Or just add 'Z'?
            d = new Date(dateStr + "Z");
        }

        if (!isNaN(d.getTime())) {
            day = d.getDate();
            month = d.getMonth();
            year = d.getFullYear();
            if (mode === "datetime") {
                hour = d.getHours();
                minute = d.getMinutes();
                second = d.getSeconds();
                // Fractional shouldn't change on TZ conversion usually unless it adds/removes precision,
                // but JS Date doesn't really handle microseconds well (only milliseconds).
                // We'll keep the existing fractional part or try to extract milliseconds?
                // Let's just keep the fractional part as is for visual consistency unless we really want
                // to support milli extraction.
                // Actually, if we convert, we might lose precision if we rely on .getMilliseconds().
                // Safest is to leave fractional as is or just clear it? User probably wants to keep it.
            }
            // After converting to local representation, the timezone part "disappears" or becomes local implication
            timezone = "";
        }
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

        // Re-append timezone if it exists
        const tz = timezone || "";
        const frac = fractional ? `.${fractional}` : "";
        onCommit(
            `${y}-${pad(m)}-${pad(d)}T${pad(useHour)}:${pad(useMinute)}:${pad(useSecond)}${frac}${tz}`,
        );
    }
</script>

<PopoverShell {anchorEl} {onCancel} minWidth={280} maxWidth={380}>
    <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
    <div class="flex flex-col gap-3 p-3" role="group" onkeydown={handleKeydown}>
        <!-- Single Line Segmented Input -->
        <div class="flex flex-col gap-1.5">
            <span
                class="text-[10px] uppercase font-bold text-muted-foreground/60 tracking-wider font-mono"
            >
                {mode === "datetime" ? "Date & Time" : "Date"}
            </span>
            <div
                class="flex items-center rounded-md border border-input bg-transparent focus-within:ring-1 focus-within:ring-ring focus-within:border-ring transition-all h-9 px-2 w-full"
            >
                <!-- Date Parts -->
                <input
                    type="number"
                    class="w-[2.5ch] bg-transparent text-center text-sm outline-none placeholder:text-muted-foreground/30 [appearance:textfield] [&::-webkit-outer-spin-button]:appearance-none [&::-webkit-inner-spin-button]:appearance-none p-0"
                    bind:value={day}
                    min="1"
                    max="31"
                    placeholder="DD"
                />
                <span
                    class="text-muted-foreground/30 mx-0.5 select-none font-light"
                    >/</span
                >
                <input
                    type="number"
                    class="w-[2.5ch] bg-transparent text-center text-sm outline-none placeholder:text-muted-foreground/30 [appearance:textfield] [&::-webkit-outer-spin-button]:appearance-none [&::-webkit-inner-spin-button]:appearance-none p-0"
                    bind:value={displayMonth}
                    min="1"
                    max="12"
                    placeholder="MM"
                />
                <span
                    class="text-muted-foreground/30 mx-0.5 select-none font-light"
                    >/</span
                >
                <input
                    type="number"
                    class="min-w-[4ch] w-[4ch] bg-transparent text-center text-sm outline-none placeholder:text-muted-foreground/30 [appearance:textfield] [&::-webkit-outer-spin-button]:appearance-none [&::-webkit-inner-spin-button]:appearance-none p-0"
                    bind:value={year}
                    min="1900"
                    max="2100"
                    placeholder="YYYY"
                />

                <!-- Time Parts (if datetime) -->
                {#if mode === "datetime"}
                    <div class="h-4 w-px bg-border/60 mx-3"></div>

                    <input
                        type="number"
                        class="w-[2.5ch] bg-transparent text-center text-sm outline-none placeholder:text-muted-foreground/30 [appearance:textfield] [&::-webkit-outer-spin-button]:appearance-none [&::-webkit-inner-spin-button]:appearance-none p-0"
                        bind:value={hour}
                        min="0"
                        max="23"
                        placeholder="HH"
                    />
                    <span
                        class="text-muted-foreground/30 mx-0.5 select-none font-light"
                        >:</span
                    >
                    <input
                        type="number"
                        class="w-[2.5ch] bg-transparent text-center text-sm outline-none placeholder:text-muted-foreground/30 [appearance:textfield] [&::-webkit-outer-spin-button]:appearance-none [&::-webkit-inner-spin-button]:appearance-none p-0"
                        bind:value={minute}
                        min="0"
                        max="59"
                        placeholder="MM"
                    />
                    <span
                        class="text-muted-foreground/30 mx-0.5 select-none font-light"
                        >:</span
                    >
                    <input
                        type="number"
                        class="w-[2.5ch] bg-transparent text-center text-sm outline-none placeholder:text-muted-foreground/30 [appearance:textfield] [&::-webkit-outer-spin-button]:appearance-none [&::-webkit-inner-spin-button]:appearance-none p-0"
                        bind:value={second}
                        min="0"
                        max="59"
                        placeholder="SS"
                    />
                    <span
                        class="text-muted-foreground/30 mx-0.5 select-none font-light"
                        >.</span
                    >
                    <input
                        type="text"
                        inputmode="numeric"
                        pattern="[0-9]*"
                        class="w-[6ch] bg-transparent text-left text-sm outline-none placeholder:text-muted-foreground/30 [appearance:textfield] p-0"
                        bind:value={fractional}
                        placeholder="ssssss"
                        oninput={(e) => {
                            const val = e.currentTarget.value;
                            e.currentTarget.value = val.replace(/[^0-9]/g, "");
                            fractional = e.currentTarget.value;
                        }}
                    />
                {/if}
                {#if timezone}
                    <span
                        class="text-muted-foreground/50 ml-2 text-xs font-mono"
                        >{timezone}</span
                    >
                {/if}
            </div>
        </div>

        <!-- Actions Row -->
        <div class="flex items-center gap-3 pt-2 px-1">
            <button
                type="button"
                class="rounded-sm text-[11px] font-medium text-muted-foreground/80 hover:text-foreground hover:bg-muted/40 transition-colors px-1.5 py-0.5"
                onclick={() => {
                    const now = new Date();
                    day = now.getDate();
                    month = now.getMonth();
                    year = now.getFullYear();
                    if (mode === "datetime") {
                        hour = now.getHours();
                        minute = now.getMinutes();
                        second = now.getSeconds();
                    }
                }}
            >
                Now
            </button>
            <button
                type="button"
                class="rounded-sm text-[11px] font-medium text-muted-foreground/80 hover:text-foreground hover:bg-muted/40 transition-colors px-1.5 py-0.5"
                onclick={convertToLocal}
            >
                Local
            </button>
            <button
                type="button"
                class="rounded-sm text-[11px] font-medium text-muted-foreground/80 hover:text-foreground hover:bg-muted/40 transition-colors px-1.5 py-0.5"
                onclick={() => {
                    // Clean reset
                    const {
                        day: d,
                        month: m,
                        year: y,
                        hour: h,
                        minute: min,
                        second: s,
                        fractional: frac,
                        timezone: tz,
                    } = parseIncoming(value);
                    day = d;
                    month = m;
                    year = y;
                    hour = h;
                    minute = min;
                    second = s;
                    fractional = frac;
                    timezone = tz;
                }}
            >
                Reset
            </button>
            <button
                type="button"
                class="ml-auto rounded-sm text-[11px] font-medium text-muted-foreground/80 hover:text-destructive hover:bg-destructive/10 transition-colors px-1.5 py-0.5"
                onclick={() => {
                    day = 1;
                    month = 0;
                    year = new Date().getFullYear();
                    hour = 0;
                    minute = 0;
                    second = 0;
                    fractional = "";
                }}
            >
                Clear
            </button>
        </div>

        <div
            class="flex items-center justify-center gap-2 mt-1 border-t border-border/40 pt-2"
        >
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
