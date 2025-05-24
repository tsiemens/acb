import { loadText } from "./http_utils.js";

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
