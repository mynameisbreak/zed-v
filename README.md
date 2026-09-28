# Zed V

This extension adds support for the [V programming language](https://vlang.org/) to the [Zed editor](https://zed.dev).

## Installation

1. Open Zed
2. Press `ctrl`+`shift`+`x` or `cmd`+`shift`+`x` to open the extension menu
3. Search for `v` and click on install

## Language Server Support

This extension launches [`VLS`](https://github.com/vlang/vls).

Lookup order:

1. `VLS_PATH` environment variable (absolute path to `vls` / `vls.exe`)
2. `vls` on `PATH`
3. `vls` next to the `v` compiler (`VLS_V_COMMAND` or `v` on `PATH`)
4. Download a prebuilt binary from [`lv37/vls`](https://github.com/lv37/vls)

If the download keeps failing, install VLS manually and make sure `vls` is on `PATH`, or set `VLS_PATH`.

The extension also sets `VLS_V_COMMAND` automatically when it can find the V compiler, so diagnostics and completion can call `v` even if the editor process has a different `PATH` than your shell.

### Zed settings (optional)

Language server id is `v` (same as the extension id):

```json
{
  "lsp": {
    "v": {
      "binary": {
        "path": "C:/path/to/vls.exe",
        "env": {
          "VLS_V_COMMAND": "C:/path/to/v.exe"
        }
      }
    }
  },
  "languages": {
    "V": {
      "language_servers": ["v"]
    }
  }
}
```

## Runnables

By default `runnables` won't do anything as they only run appropriately tagged tasks.
To use runnables, you must add one of these supported tags to your tasks:
 - `v-main`: Runs on the `main` function in a V file.
 - `v-test`: Runs on functions whose names start with `test_`

## Example

Some example tasks to get you started. Place these in your `tasks.json` file.
```json
[
  {
    "label": "V run main",
    "command": "v",
    "args": ["run", "$ZED_FILE"],
    "tags": ["v-main"],
    "use_new_terminal": false,
    "reveal": "always"
  },
  {
    "label": "V test",
    "command": "v",
    "args": ["test", "$ZED_DIRNAME"],
    "tags": ["v-test"],
    "use_new_terminal": false,
    "reveal": "always"
  }
]
```
