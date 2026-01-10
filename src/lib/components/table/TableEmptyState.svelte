<script lang="ts">
    interface Props {
        title?: string;
        description?: string;
    }

    let {
        title = "No results",
        description = "Your query returned no data",
    }: Props = $props();
</script>

<div
    class="flex flex-col items-center justify-center h-full w-full py-16 gap-6 text-muted-foreground select-none"
>
    <!-- Animated Database/Table Illustration -->
    <div class="relative">
        <!-- Subtle glow effect -->
        <div
            class="absolute inset-0 blur-xl opacity-20 bg-linear-to-br from-primary/30 to-accent/30 rounded-full scale-150 animate-pulse-slow"
        ></div>

        <!-- Main illustration -->
        <svg
            width="120"
            height="120"
            viewBox="0 0 120 120"
            fill="none"
            xmlns="http://www.w3.org/2000/svg"
            class="relative z-10"
        >
            <!-- Outer circle -->
            <circle
                cx="60"
                cy="60"
                r="56"
                stroke="currentColor"
                stroke-width="1.5"
                stroke-dasharray="4 4"
                class="opacity-30"
            />

            <!-- Table icon base -->
            <g class="opacity-60">
                <!-- Table surface -->
                <rect
                    x="28"
                    y="36"
                    width="64"
                    height="48"
                    rx="4"
                    stroke="currentColor"
                    stroke-width="2"
                    fill="none"
                />

                <!-- Header row -->
                <line
                    x1="28"
                    y1="52"
                    x2="92"
                    y2="52"
                    stroke="currentColor"
                    stroke-width="2"
                />

                <!-- Vertical dividers -->
                <line
                    x1="48"
                    y1="36"
                    x2="48"
                    y2="84"
                    stroke="currentColor"
                    stroke-width="1.5"
                    class="opacity-50"
                />
                <line
                    x1="72"
                    y1="36"
                    x2="72"
                    y2="84"
                    stroke="currentColor"
                    stroke-width="1.5"
                    class="opacity-50"
                />

                <!-- Data rows -->
                <line
                    x1="28"
                    y1="68"
                    x2="92"
                    y2="68"
                    stroke="currentColor"
                    stroke-width="1"
                    class="opacity-30"
                />
            </g>

            <!-- Animated dots indicating empty data -->
            <g class="animate-fade-stagger">
                <circle
                    cx="38"
                    cy="60"
                    r="2"
                    fill="currentColor"
                    class="opacity-40"
                    style="animation-delay: 0s"
                />
                <circle
                    cx="60"
                    cy="60"
                    r="2"
                    fill="currentColor"
                    class="opacity-40"
                    style="animation-delay: 0.2s"
                />
                <circle
                    cx="82"
                    cy="60"
                    r="2"
                    fill="currentColor"
                    class="opacity-40"
                    style="animation-delay: 0.4s"
                />
            </g>

            <!-- Question mark overlay -->
            <text
                x="60"
                y="78"
                text-anchor="middle"
                font-size="16"
                font-weight="600"
                fill="currentColor"
                class="opacity-50"
            >
                ?
            </text>
        </svg>
    </div>

    <!-- Text content -->
    <div class="flex flex-col items-center gap-2 text-center max-w-xs">
        <h3 class="text-base font-medium text-foreground/80">{title}</h3>
        <p class="text-sm opacity-60 leading-relaxed">{description}</p>
    </div>
</div>

<style>
    .animate-pulse-slow {
        animation: pulse 3s ease-in-out infinite;
    }

    .animate-fade-stagger circle {
        animation: fade-dot 1.5s ease-in-out infinite;
    }

    .animate-fade-stagger circle:nth-child(1) {
        animation-delay: 0s;
    }
    .animate-fade-stagger circle:nth-child(2) {
        animation-delay: 0.2s;
    }
    .animate-fade-stagger circle:nth-child(3) {
        animation-delay: 0.4s;
    }

    @keyframes pulse {
        0%,
        100% {
            opacity: 0.15;
            transform: scale(1.4);
        }
        50% {
            opacity: 0.25;
            transform: scale(1.6);
        }
    }

    @keyframes fade-dot {
        0%,
        100% {
            opacity: 0.2;
        }
        50% {
            opacity: 0.6;
        }
    }
</style>
