export function isDebugModeEnabled(): boolean {
    const hostIsDebug = window.location.hostname === "localhost" ||
        window.location.hostname.match(/^\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3}$/) !== null;
    const debugModeDisabled = window.location.search.includes("debug=false");
    return hostIsDebug && !debugModeDisabled;
}
