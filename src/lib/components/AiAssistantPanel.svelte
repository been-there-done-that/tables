<script lang="ts">
    import IconX from "@tabler/icons-svelte/icons/x";
    import IconBrandTesla from "@tabler/icons-svelte/icons/brand-tesla";
    import { windowState } from "$lib/stores/window.svelte";
    // Picasso-inspired Cubist AI Agent Teaser
    const aiLabSvg = `
    <svg viewBox="0 0 800 600" xmlns="http://www.w3.org/2000/svg" class="w-full h-full">
        <defs>
            <filter id="painterly" x="-20%" y="-20%" width="140%" height="140%">
                <feTurbulence type="fractalNoise" baseFrequency="0.03" numOctaves="3" result="noise" />
                <feDisplacementMap in="SourceGraphic" in2="noise" scale="5" />
            </filter>
            <marker id="arrow" markerWidth="10" markerHeight="10" refX="9" refY="3" orient="auto">
                <path d="M0,0 L0,6 L9,3 z" fill="#2C2C2C" />
            </marker>
        </defs>

        <!-- Background: Muted Neutral -->
        <rect width="800" height="600" fill="#F2F0E9" />

        <!-- Layer 1: Background Shapes (Abstract Geometry) -->
        <path d="M50,300 Q150,50 300,100 T500,200" fill="none" stroke="#D9A441" stroke-width="2" opacity="0.5" />
        <circle cx="600" cy="150" r="80" fill="#7C9060" opacity="0.3" />
        <rect x="100" y="400" width="150" height="150" fill="#5D7C89" opacity="0.2" transform="rotate(-15 175 475)" />

        <!-- Main Composition Group -->
        <g filter="url(#painterly)" stroke="#2C2C2C" stroke-width="3" stroke-linecap="round" stroke-linejoin="round">
            
            <!-- Fragmented Database Cylinder (The Core) -->
            <!-- Top plane -->
            <ellipse cx="400" cy="250" rx="60" ry="30" fill="#D9A441" />
            <!-- Broken side bodies -->
            <path d="M340,250 L340,400 L400,430 L400,280 Z" fill="#8B5A2B" />
            <path d="M400,280 L400,430 L460,400 L460,250 Z" fill="#5D7C89" opacity="0.9" />
            
            <!-- Floating Cube Planes (Cubist Fragmentation) -->
            <!-- Front face, distorted -->
            <path d="M280,350 L350,450 L250,500 L180,400 Z" fill="#D9A441" opacity="0.8" />
            <!-- Side plane, intersecting -->
            <path d="M320,380 L450,380 L420,520 L300,500 Z" fill="#7C9060" opacity="0.7" />
            
            <!-- The "Eye" / Intelligence (Abstract) -->
            <path d="M380,180 Q400,150 420,180 T400,210 Z" fill="#F2F0E9" stroke-width="4" />
            <circle cx="400" cy="180" r="8" fill="#2C2C2C" />
            
            <!-- Half-built Structures / Exposed Gears -->
            <path d="M500,300 L550,300 L550,350 L520,350" fill="none" />
            <path d="M550,325 L600,325" fill="none" marker-end="url(#arrow)" />
            <rect x="520" y="400" width="40" height="60" fill="#2C2C2C" opacity="0.1" />
            <path d="M520,400 L560,460" stroke="#2C2C2C" stroke-width="1" />
            
            <!-- Floating Schema Nodes -->
            <rect x="200" y="150" width="60" height="40" fill="#5D7C89" />
            <path d="M260,170 L350,200" stroke-dasharray="5,5" />
            
            <rect x="550" y="200" width="50" height="50" fill="#8B5A2B" transform="rotate(10)" />
            <path d="M460,250 L550,225" />

            <!-- Dynamic Wires/Connections ("Still Cooking") -->
            <path d="M400,430 C350,500 450,500 400,550" fill="none" stroke="#D54E4E" stroke-width="4" />
            <path d="M180,400 C150,350 200,300 150,250" fill="none" stroke="#2C2C2C" stroke-width="2" />

        </g>
        
        <!-- Texture Overlay (Painterly Feel) -->
        <rect width="100%" height="100%" filter="url(#painterly)" opacity="0.15" fill="#D9A441" style="mix-blend-mode: overlay;" />

    </svg>
    `;
</script>

<div class="flex h-full w-full flex-col bg-background">
    <!-- Header -->
    <div
        class="h-8 flex items-center justify-between px-3 py-1 border-b border-border bg-muted/30"
    >
        <div class="flex items-center gap-2">
            <IconBrandTesla class="size-4 text-red-500" />
            <h2 class="text-xs font-semibold text-muted-foreground">
                AI Assistant
            </h2>
        </div>
        <button
            type="button"
            class="h-6 w-6 flex items-center justify-center hover:bg-accent rounded text-muted-foreground transition-colors"
            onclick={() => windowState.closeRightPanel()}
        >
            <IconX class="size-4" />
        </button>
    </div>

    <!-- Content -->
    <div
        class="flex-1 overflow-auto flex flex-col items-center justify-center p-6 text-center space-y-6"
    >
        <div class="relative w-64 h-64 opacity-90">
            {@html aiLabSvg}
        </div>

        <div class="space-y-2 max-w-xs">
            <h3 class="text-lg font-medium tracking-tight">
                Cooking Intelligence...
            </h3>
            <p class="text-sm text-muted-foreground">
                We are currently brewing the AI in the lab. Please check back
                later for advanced query assistance.
            </p>
        </div>

        <!-- Loading Indicator Style -->
        <div class="flex gap-1.5">
            <div class="h-2 w-2 animate-bounce rounded-full bg-red-500"></div>
            <div
                class="h-2 w-2 animate-bounce rounded-full bg-red-500 delay-150"
            ></div>
            <div
                class="h-2 w-2 animate-bounce rounded-full bg-red-500 delay-300"
            ></div>
        </div>
    </div>
</div>
