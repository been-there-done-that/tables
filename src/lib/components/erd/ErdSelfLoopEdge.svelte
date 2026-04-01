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
    const offset = 80;
    // source = right side of FK column, target = left side of PK column (same node).
    // Both control points anchor to sourceX so the curve bulges rightward
    // instead of crossing through the node body.
    const path = $derived(`M ${sourceX} ${sourceY} C ${sourceX + offset} ${sourceY}, ${sourceX + offset} ${targetY}, ${targetX} ${targetY}`);
</script>

<BaseEdge {id} {path} {markerEnd} {style} />
