<script>
    import "../app.css";
    import { themeRegistry } from '../lib/themes/registry';
    import { onMount } from 'svelte';

    let { children } = $props();

    // Initialize theme registry on app start
    onMount(async () => {
        try {
            await themeRegistry.init();
            console.log('Theme system initialized');
        } catch (error) {
            console.error('Failed to initialize theme system:', error);
        }
    });
</script>

<div class="min-h-screen bg-bg-primary text-fg-primary">
    {@render children()}
</div>
