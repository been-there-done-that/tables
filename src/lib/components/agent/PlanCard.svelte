<!-- src/lib/components/agent/PlanCard.svelte -->
<script lang="ts">
    import type { AgentPlan } from "$lib/stores/plans.svelte";
    import IconChevronDown from "@tabler/icons-svelte/icons/chevron-down";
    import IconChevronRight from "@tabler/icons-svelte/icons/chevron-right";
    import IconCheck from "@tabler/icons-svelte/icons/check";
    import IconLoader2 from "@tabler/icons-svelte/icons/loader-2";
    import IconX from "@tabler/icons-svelte/icons/x";
    import IconClock from "@tabler/icons-svelte/icons/clock";

    interface Props {
        plan: AgentPlan;
    }
    let { plan }: Props = $props();
    let expanded = $state(false);

    const doneCount = $derived(plan.steps.filter((s) => s.status === "done").length);
    const totalCount = $derived(plan.steps.length);

    const PHASE_LABEL: Record<string, string> = {
        gather: "Gather",
        draft: "Draft",
        execute: "Execute",
    };
    const PHASE_COLOR: Record<string, string> = {
        gather: "text-blue-400/70 bg-blue-400/10",
        draft: "text-purple-400/70 bg-purple-400/10",
        execute: "text-amber-400/70 bg-amber-400/10",
    };
</script>

<div class="my-1 rounded border border-amber-400/20 bg-amber-400/5 text-[11px]">
    <!-- Header -->
    <button
        onclick={() => (expanded = !expanded)}
        class="flex w-full items-center gap-2 px-2.5 py-1.5 text-left"
    >
        <span class="text-[10px] font-semibold text-amber-400/80 uppercase tracking-wide">Plan</span>
        {#if totalCount > 0}
            <span class="text-[9.5px] text-muted-foreground/50">{doneCount}/{totalCount} steps</span>
        {/if}
        <span class="flex-1"></span>
        {#if expanded}
            <IconChevronDown size={10} class="text-muted-foreground/40" />
        {:else}
            <IconChevronRight size={10} class="text-muted-foreground/40" />
        {/if}
    </button>

    <!-- Steps -->
    {#if expanded && plan.steps.length > 0}
        <div class="border-t border-amber-400/15 px-2.5 py-1.5 flex flex-col gap-0.5">
            {#each plan.steps as step (step.id)}
                <div class="flex items-start gap-2 py-0.5">
                    <!-- Status icon -->
                    <div class="mt-0.5 shrink-0">
                        {#if step.status === "done"}
                            <IconCheck size={10} class="text-green-500/80" />
                        {:else if step.status === "running"}
                            <IconLoader2 size={10} class="animate-spin text-accent" />
                        {:else if step.status === "error"}
                            <IconX size={10} class="text-destructive/70" />
                        {:else if step.status === "pending"}
                            <IconClock size={10} class="text-muted-foreground/30" />
                        {:else}
                            <span class="block h-2.5 w-2.5"></span>
                        {/if}
                    </div>
                    <!-- Phase badge -->
                    <span class="shrink-0 rounded px-1 py-0.5 text-[9px] font-medium {PHASE_COLOR[step.phase] ?? ''}">
                        {PHASE_LABEL[step.phase] ?? step.phase}
                    </span>
                    <!-- Description -->
                    <span class="text-[10.5px] text-foreground/70 leading-relaxed">{step.description}</span>
                </div>
            {/each}
        </div>
    {/if}
</div>
