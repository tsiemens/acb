export type JSONValue = string | number | boolean | null | object | Array<JSONValue>;

 /**
 * Loads JSON from a URL
 * @param {string} url - The URL to fetch JSON data from
 * @returns {Promise<JSONValue>} - Promise resolving to
 */
export function loadJSON(url: string): Promise<JSONValue> {
   return new Promise<JSONValue>((resolve, reject) => {
      let http_request = new XMLHttpRequest();

      http_request.onreadystatechange = function() {
         const DONE = 4;
         if (http_request.readyState === DONE) {
            if (http_request.status >= 200 && http_request.status < 300) {
               try {
                  // eslint-disable-next-line @typescript-eslint/no-unsafe-assignment
                  const jsonObj = JSON.parse(http_request.responseText);

                  // Otherwise return the plain JSON object
                  resolve(jsonObj as JSONValue);
               } catch (e) {
                  // Convert to Error if not one
                  if (e instanceof Error) {
                     reject(e);
                  } else {
                     // eslint-disable-next-line @typescript-eslint/restrict-template-expressions
                     reject(new Error(`Failed to parse JSON: ${e}`));
                  }
               }
            } else {
               reject(new Error(`Request failed with status ${http_request.status.toString()}: ${http_request.statusText}`));
            }
         }
      };

      http_request.open("GET", url, true);
      http_request.send();
   });
}