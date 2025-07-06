declare module 'toml' {
    export function parse(str: string): Record<string, unknown>;
}