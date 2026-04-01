<script lang="ts">
    import { BaseEdge } from '@xyflow/svelte';

    interface Props {
        id: string;
        sourceX: number;
        sourceY: number;
        targetX: number;
        targetY: number;
        markerEnd?: string;
        style?: string;
    }
    let {
        id,
        sourceX,
        sourceY,
        targetX,
        targetY,
        markerEnd = '',
        style = '',
    }: Props = $props();

    // Both source and target are on the right side of the same node.
    // Bulge 70px to the right so the loop is clearly visible outside the node.
    const offset = 70;
    const path = `M ${sourceX} ${sourceY} C ${sourceX + offset} ${sourceY}, ${targetX + offset} ${targetY}, ${targetX} ${targetY}`;
</script>

<BaseEdge {id} {path} {markerEnd} {style} />
