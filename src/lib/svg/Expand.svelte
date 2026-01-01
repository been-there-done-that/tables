<script lang="ts">
    let { class: className = undefined, ...rest } = $props();
</script>

<svg
    xmlns="http://www.w3.org/2000/svg"
    width="16"
    height="16"
    viewBox="0 0 24 24"
    fill="none"
    stroke="currentColor"
    stroke-width="2"
    stroke-linecap="round"
    stroke-linejoin="round"
    class={className}
    {...rest}
    ><path stroke="none" d="M0 0h24v24H0z" fill="none" /><path
        d="M8 9l4 -4l4 4"
    /><path d="M16 15l-4 4l-4 -4" /></svg
>
