<script lang="ts">
    import splashBg from "$lib/assets/splash_bg.png";
    import "../../app.css"; // Ensure global styles are applied
</script>

<svelte:head>
    <style>
        html,
        body {
            background: transparent !important;
        }
    </style>
</svelte:head>

<div
    class="fixed inset-0 flex items-center justify-center bg-transparent select-none overflow-hidden"
    data-tauri-drag-region
>
    <div
        class="relative w-full h-full overflow-hidden border border-border rounded-xl"
        data-tauri-drag-region
    >
        <!-- Background Image -->
        <img
            src={splashBg}
            alt="Splash Screen"
            class="absolute inset-0 w-full h-full object-cover pointer-events-none"
        />

        <!-- Overlay Gradient -->
        <div
            class="absolute inset-0 bg-gradient-to-t from-black/60 via-transparent to-transparent opacity-80 pointer-events-none"
        ></div>

        <!-- Content -->
        <div
            class="absolute bottom-10 left-10 text-white z-10 pointer-events-none"
        >
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

        <!-- Subtle accent line -->
        <div
            class="absolute bottom-0 left-0 right-0 h-1 bg-accent pointer-events-none"
        ></div>
    </div>
</div>

<style>
    /* Ensure fonts match the app theme */
    h1 {
        font-family: var(--font-family, sans-serif);
    }
</style>
