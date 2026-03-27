import { loadText } from "./http_utils.js";

export function isDebugModeEnabled(): boolean {
    const hostIsDebug = window.location.hostname === "localhost" ||
        window.location.hostname.match(/^\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3}$/) !== null;
    const debugModeDisabled = window.location.search.includes("debug=false");
    return hostIsDebug && !debugModeDisabled;
}

class TestFile {
    constructor(
        public name: string,
        public contents: string,
    ) {}
}

export function loadTestFile(onLoad: (_: TestFile) => void) {
    loadText("/samples/sample_txs.csv").then((text) => {
        onLoad(new TestFile("sample_txs.csv", text));
    }).catch(() => {});
}
