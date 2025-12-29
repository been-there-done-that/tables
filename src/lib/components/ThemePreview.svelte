<script lang="ts">
    import type { ThemeRecord, ThemeData } from "$lib/theme/types";

    let { theme } = $props<{ theme: ThemeRecord }>();

    const data = $derived.by(() => {
        try {
            return JSON.parse(theme.theme_data) as ThemeData;
        } catch {
            return null;
        }
    });

    const ui = $derived(data?.ui);
    const syntax = $derived(data?.syntax);
    const editor = $derived(data?.editor);
</script>

{#if ui}
    <div
        class="relative w-full aspect-[1.6/1] rounded-lg border shadow-sm overflow-hidden flex flex-col group transition-all duration-300 hover:shadow-md cursor-pointer"
        style="background: {ui.background.primary}; border-color: {ui.border
            .default};"
    >
        <!-- Titlebar Mockup -->
        <div
            class="h-7 px-2 flex items-center justify-between border-b"
            style="background: {ui.background.secondary}; border-color: {ui
                .border.subtle};"
        >
            <div class="flex gap-1.5 ml-1">
                <div class="size-2 rounded-full opacity-40 bg-red-500"></div>
                <div class="size-2 rounded-full opacity-40 bg-yellow-500"></div>
                <div class="size-2 rounded-full opacity-40 bg-green-500"></div>
            </div>

            <div
                class="h-4 w-28 rounded flex items-center px-2 opacity-60"
                style="background: {ui.background.tertiary};"
            >
                <div
                    class="h-1 w-full rounded-full"
                    style="background: {ui.foreground.tertiary};"
                ></div>
            </div>

            <div class="flex gap-2 mr-1">
                <div
                    class="size-3 rounded opacity-30"
                    style="background: {ui.foreground.tertiary};"
                ></div>
                <div
                    class="size-3 rounded-full opacity-50"
                    style="background: {ui.accent
                        .primary}; border: 1.5px solid {ui.accent.primary};"
                ></div>
            </div>
        </div>

        <!-- Layout Container -->
        <div class="flex-1 flex overflow-hidden">
            <!-- Sidebar Mockup -->
            <div
                class="w-12 border-r flex flex-col gap-2 p-2"
                style="background: {ui.background.secondary}; border-color: {ui
                    .border.subtle};"
            >
                <div
                    class="h-1.5 w-full rounded-sm opacity-20"
                    style="background: {ui.foreground.primary};"
                ></div>
                <div
                    class="h-1.5 w-2/3 rounded-sm opacity-20"
                    style="background: {ui.foreground.primary};"
                ></div>
                <div
                    class="h-1.5 w-full rounded-sm mt-1"
                    style="background: {ui.background
                        .hover}; border: 1px solid {ui.border.subtle};"
                ></div>
                <div
                    class="mt-auto h-1.5 w-full rounded-sm opacity-20"
                    style="background: {ui.foreground.primary};"
                ></div>
            </div>

            <!-- Main Content / Editor Mockup -->
            <div class="flex-1 flex flex-col overflow-hidden">
                <!-- Explorer Header / Tabs -->
                <div
                    class="h-6 px-2 flex items-center gap-1 border-b"
                    style="background: {ui.background
                        .primary}; border-color: {ui.border.subtle};"
                >
                    <div
                        class="h-full px-2 flex items-center gap-2 border-r"
                        style="background: {ui.background
                            .secondary}; border-color: {ui.border.subtle};"
                    >
                        <div
                            class="h-0.5 w-4 rounded-full"
                            style="background: {ui.foreground.tertiary};"
                        ></div>
                        <div
                            class="size-1.5 rounded-full opacity-30"
                            style="background: {ui.foreground.primary};"
                        ></div>
                    </div>
                    <div
                        class="h-full px-2 flex items-center gap-2 border-r"
                        style="background: {ui.background
                            .primary}; border-color: {ui.border.subtle};"
                    >
                        <div
                            class="h-0.5 w-5 rounded-full"
                            style="background: {ui.accent.primary};"
                        ></div>
                        <div
                            class="size-1.5 rounded-full"
                            style="background: {ui.accent.primary};"
                        ></div>
                    </div>
                </div>

                <div
                    class="flex-1 p-3 flex flex-col gap-3 overflow-hidden"
                    style="background: {editor?.background ||
                        ui.background.primary};"
                >
                    <!-- Search Bar Mockup -->
                    <div
                        class="h-6 w-full rounded-md border flex items-center px-2 gap-2"
                        style="background: {ui.background
                            .secondary}; border-color: {ui.border.default};"
                    >
                        <div
                            class="size-2.5 rounded-full opacity-40 shrink-0"
                            style="background: {ui.foreground.tertiary};"
                        ></div>
                        <div
                            class="h-1.5 w-24 rounded-full opacity-25"
                            style="background: {ui.foreground.primary};"
                        ></div>
                        <div
                            class="ml-auto h-3.5 w-7 rounded-sm shadow-sm flex items-center justify-center p-0.5"
                            style="background: {ui.accent.primary};"
                        >
                            <div
                                class="h-0.5 w-3 rounded-full opacity-50 bg-white"
                            ></div>
                        </div>
                    </div>

                    <!-- Fake Code / Content -->
                    <div class="flex flex-col gap-1.5 mt-1">
                        <div class="flex gap-1.5">
                            <div
                                class="h-1.5 w-8 rounded-full"
                                style="background: {syntax?.keyword ||
                                    ui.accent.primary};"
                            ></div>
                            <div
                                class="h-1.5 w-12 rounded-full"
                                style="background: {syntax?.variable ||
                                    ui.foreground.primary};"
                            ></div>
                            <div
                                class="h-1.5 w-4 rounded-full"
                                style="background: {syntax?.operator ||
                                    ui.accent.primary};"
                            ></div>
                            <div
                                class="h-1.5 w-16 rounded-full opacity-60"
                                style="background: {syntax?.string ||
                                    ui.foreground.secondary};"
                            ></div>
                        </div>
                        <div class="flex gap-1.5 pl-4">
                            <div
                                class="h-1.5 w-6 rounded-full"
                                style="background: {syntax?.keyword ||
                                    ui.accent.primary};"
                            ></div>
                            <div
                                class="h-1.5 w-20 rounded-full"
                                style="background: {syntax?.function ||
                                    ui.foreground.primary};"
                            ></div>
                        </div>
                        <div class="flex gap-1.5 pl-4">
                            <div
                                class="h-1.5 w-10 rounded-full opacity-30"
                                style="background: {syntax?.comment ||
                                    ui.foreground.tertiary};"
                            ></div>
                        </div>
                    </div>
                </div>

                <!-- Status Bar Mockup -->
                <div
                    class="h-4 px-2 flex items-center justify-between"
                    style="background: {ui.accent.primary};"
                >
                    <div class="flex gap-2 items-center">
                        <div
                            class="size-1.5 rounded-full bg-white opacity-50"
                        ></div>
                        <div
                            class="h-0.5 w-8 rounded-full bg-white opacity-30"
                        ></div>
                    </div>
                    <div
                        class="h-0.5 w-12 rounded-full bg-white opacity-20"
                    ></div>
                </div>
            </div>
        </div>

        <!-- Active Indicator Overlay -->
        <div
            class="absolute inset-x-0 bottom-0 h-0.5 opacity-0 group-hover:opacity-100 transition-opacity"
            style="background: {ui.accent.primary};"
        ></div>
    </div>
{/if}
