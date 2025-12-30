import type { Component } from "svelte";

// simple map for renderers
const cellRenderers = new Map<string, Component<any>>();
const editorRenderers = new Map<string, Component<any>>();

export function registerCellRenderer(key: string, component: Component<any>) {
    cellRenderers.set(key, component);
}

export function getCellRenderer(key: string) {
    return cellRenderers.get(key);
}

export function registerEditorRenderer(key: string, component: Component<any>) {
    editorRenderers.set(key, component);
}

export function getEditorRenderer(key: string) {
    return editorRenderers.get(key);
}
