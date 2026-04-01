<script lang="ts">
    import { BaseEdge } from '@xyflow/svelte';
    import type { SelfLoopData } from './erd-layout';

    interface Props {
        id: string;
        sourceX: number;
        sourceY: number;
        targetX: number;
        targetY: number;
        markerEnd?: string;
        style?: string;
        data?: SelfLoopData;
    }
    let { id, sourceX, sourceY, targetX, targetY, markerEnd = '', style = '', data }: Props = $props();

    // Fan each self-loop further left so multiple loops on the same table don't overlap.
    // First loop = 50px out, each subsequent one adds 25px more.
    const BASE_OFFSET = 50;
    const SPACING = 25;
    const offset = $derived(BASE_OFFSET + (data?.selfLoopIndex ?? 0) * SPACING);

    // Orthogonal step path that stays fully to the LEFT of the node:
    // exit left → step further left (fanned) → step vertically to PK row → step right back to target.
    const path = $derived(
        `M ${sourceX} ${sourceY} H ${sourceX - offset} V ${targetY} H ${targetX}`
    );
</script>

<BaseEdge {id} {path} {markerEnd} {style} />
