export const NULL_TOKEN = "<null>";
export const DEFAULT_TOKEN = "<default>";

export type BooleanLike =
    | true
    | false
    | null
    | typeof NULL_TOKEN
    | typeof DEFAULT_TOKEN
    | undefined;

export function normalizeIncomingBoolean(val: any): BooleanLike {
    if (val === DEFAULT_TOKEN) return DEFAULT_TOKEN;
    if (val === NULL_TOKEN || val === null) return NULL_TOKEN;
    if (val === true || val === false) return val;
    return val as BooleanLike;
}

export function commitBooleanValue(val: BooleanLike): any {
    if (val === NULL_TOKEN) return null;
    if (val === DEFAULT_TOKEN) return DEFAULT_TOKEN;
    if (val === true || val === false) return val;
    return val;
}

export function displayBooleanValue(val: BooleanLike): string {
    const normalized = normalizeIncomingBoolean(val);
    if (normalized === NULL_TOKEN) return "<null>";
    if (normalized === DEFAULT_TOKEN) return "<default>";
    return String(normalized ?? "");
}
