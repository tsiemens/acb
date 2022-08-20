DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" >/dev/null 2>&1 && pwd )"
IMPORTS_DIR=$DIR/html/imports/
WASM_EXEC_FILE=$IMPORTS_DIR/wasm_exec.js
SOURCE_WASM_EXEC_FILE="$(go env GOROOT)/misc/wasm/wasm_exec.js"
set -x
if ! [ -f $WASM_EXEC_FILE ] || ! cmp -s $SOURCE_WASM_EXEC_FILE $WASM_EXEC_FILE; then
   mkdir -p $IMPORTS_DIR
   cp $SOURCE_WASM_EXEC_FILE $IMPORTS_DIR
fi
