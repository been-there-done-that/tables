<script lang="ts">
    import splashBg from "$lib/assets/splash_bg.png";
    import { fade } from "svelte/transition";

    let { visible = true } = $props();
</script>

{#if visible}
    <div
        class="fixed inset-0 z-[9999] flex items-center justify-center bg-background select-none"
        transition:fade={{ duration: 400 }}
    >
        <div
            class="relative w-[600px] h-[350px] shadow-2xl rounded-lg overflow-hidden border border-border"
        >
            <!-- Background Image -->
            <img
                src={splashBg}
                alt="Splash Screen"
                class="absolute inset-0 w-full h-full object-cover"
            />

            <!-- Overlay Gradient for text readability if needed -->
            <div
                class="absolute inset-0 bg-gradient-to-t from-black/60 via-transparent to-transparent opacity-80"
            ></div>

            <!-- Content -->
            <div class="absolute bottom-10 left-10 text-white z-10">
                <h1 class="text-4xl font-bold tracking-tight mb-2 font-mono">
                    Tables
                </h1>
                <div class="flex items-center gap-2 opacity-80">
                    <span class="text-sm font-medium tracking-wider uppercase"
                        >Database Client</span
                    >
                    <span class="w-1 h-1 rounded-full bg-white/50"></span>
                    <span class="text-sm font-mono">v0.1.0</span>
                </div>
            </div>

            <!-- Subtle accent line at bottom like IntelliJ -->
            <div class="absolute bottom-0 left-0 right-0 h-1 bg-accent"></div>
        </div>
    </div>
{/if}

<style>
    /* Ensure fonts match the app theme */
    h1 {
        font-family: var(--font-family, sans-serif);
    }
</style>
