# Compiler Explorer REST API — Reference Notes

Internal reference for llvm-airfryer development. Based on the CE source code
at `~/.llvm_airfryer/compiler-explorer/`.

## API Base

All endpoints are under `/api/`. When running locally: `http://localhost:10240/api/`.

Defined in:
- `lib/handlers/api.ts` — main API handler setup
- `lib/handlers/route-api.ts` — shortlink and clientstate routes
- `lib/handlers/compile.ts` — compilation logic

## Compilation

### POST `/api/compiler/<compiler-id>/compile`

Compile source code and return assembly output.

**Source:** `lib/handlers/api.ts:96-98`

Two input formats:

**JSON format** (`Content-Type: application/json`):
```json
{
  "source": "#include <stdio.h>\nint main() { return 0; }",
  "options": {
    "userArguments": "-O2 -march=znver4",
    "compilerOptions": {
      "skipAsm": false,
      "executorRequest": false
    },
    "filters": {
      "binary": false,
      "binaryObject": false,
      "commentOnly": true,
      "demangle": true,
      "directives": true,
      "execute": false,
      "intel": true,
      "labels": true,
      "libraryCode": false,
      "trim": false
    },
    "tools": [],
    "libraries": []
  },
  "lang": "c++",
  "files": [
    {
      "filename": "header.h",
      "contents": "#define VALUE 42"
    }
  ],
  "allowStoreCodeDebug": true
}
```

**Text format** (`Content-Type: text/plain`):
```bash
curl -s "http://localhost:10240/api/compiler/clang_trunk/compile" \
  -H "Content-Type: text/plain" \
  -d "int main() { return 42; }" \
  --data-urlencode "options=-O2"
```

**Response** includes:
- `asm` — array of `{text, source}` objects (assembly lines with source mapping)
- `code` — exit code
- `stdout`, `stderr` — compiler output
- `compilationOptions` — actual flags used
- `tools` — tool results if any were requested

### POST `/api/compiler/<compiler-id>/cmake`

Compile CMake projects.

**Source:** `lib/handlers/api.ts:100`

### POST `/api/format/<formatter>`

Format source code using clang-format or other formatters.

**Source:** `lib/handlers/api.ts:107-109`

```json
{
  "source": "int main(){return 0;}",
  "base": "Google"
}
```

## Compiler & Language Discovery

### GET `/api/languages`

List all supported languages.

**Source:** `lib/handlers/api.ts:77`

### GET `/api/compilers`

List all available compilers. Supports field filtering: `?fields=id,name,lang`.

**Source:** `lib/handlers/api.ts:79-81`

### GET `/api/compilers/<language-id>`

List compilers for a specific language (e.g., `c++`, `c`, `zig`).

### GET `/api/libraries/<language-id>`

List available libraries for a language.

**Source:** `lib/handlers/api.ts:83-89`

### GET `/api/tools/<language-id>`

List tools available for a language.

### GET `/api/formats`

List available code formatters.

**Source:** `lib/handlers/api.ts:105`

### GET `/api/asm/<instructionSet>/<opcode>`

Get opcode documentation (e.g., `/api/asm/amd64/vpternlogq`).

**Source:** `lib/handlers/api.ts:113`

## Shortlinks & Session State

### POST `/api/shortener`

Save a full CE session (ClientState) and get a persistent short link.

**Source:** `lib/handlers/api.ts:115-116`

**Request body:** ClientState JSON (source code, compiler selection, flags, layout)

**Response:**
```json
{
  "url": "http://localhost:10240/z/abc123"
}
```

### GET `/api/shortlinkinfo/<id>`

Retrieve full ClientState JSON for a shortlink.

**Source:** `lib/handlers/api.ts:118`

**Response:** Complete session state including source code, selected compilers,
options, and UI layout. This is the primary way to "read back" what a user
has in their CE session.

### GET `/z/<id>`

Open the CE website with the saved session loaded.

**Source:** `lib/handlers/route-api.ts:76-81`

### GET `/z/<id>/code/<sourceid>`

Get just the source code from a shortlink session as **plain text**.
`sourceid` is 1-indexed (1 for first editor, 2 for second, etc.)

**Source:** `lib/handlers/route-api.ts:83-105`

### GET `/z/<id>/resetlayout`

Open shortlink with a reset layout.

**Source:** `lib/handlers/route-api.ts:107`

## ClientState URL Encoding

### GET `/clientstate/<base64>`

Open CE with a pre-loaded state encoded in the URL. The base64 can be:
- Plain base64-encoded JSON
- Gzip-compressed then base64-encoded JSON (for larger states)

**Source:** `lib/handlers/route-api.ts:152-172`, `lib/clientstate.ts`

This is the best way to "send code to CE" — construct a ClientState JSON,
base64-encode it, and open the URL in the browser.

**ClientState structure** (from `lib/clientstate.ts`):
```json
{
  "sessions": [
    {
      "id": 1,
      "language": "c++",
      "source": "int main() { return 0; }",
      "compilers": [
        {
          "id": "clang_trunk",
          "options": "-O2 -march=znver4",
          "filters": {
            "binary": false,
            "commentOnly": true,
            "demangle": true,
            "directives": true,
            "intel": true,
            "labels": true,
            "trim": false
          }
        }
      ]
    }
  ]
}
```

## Metadata

### GET `/api/version`

Returns the CE version (git release name).

### GET `/api/releaseBuild`

Returns the release build number.

## Configuration

From `lib/handlers/api.ts:70`:
- Cache: `public, max-age=<apiMaxAgeSecs>` (default 24 hours / 86400s)
- Max upload: configurable (default 1MB)
- Accept header determines response format: JSON or plain text

## Full API Documentation

CE ships its own API docs at:
- `docs/API.md` in the CE source tree (~19KB)
- Also served at `/api` when `Accept: text/html` is requested

## Potential llvm-airfryer Integrations

### Send code to CE (high value)
1. Read a local `.c`/`.cpp` file
2. Construct a ClientState with source + our custom compilers
3. Base64-encode → open `http://localhost:<port>/clientstate/<encoded>` in browser
4. User sees their code with our custom LLVM compilers pre-selected

### Compile from terminal (high value)
1. `POST /api/compiler/<id>/compile` with source from a file
2. Display assembly output in terminal (or pipe to file)
3. Could diff assembly between upstream and branch compilers

### Save/restore CE sessions
1. User creates a useful CE layout
2. `GET /api/shortlinkinfo/<id>` to capture the state
3. Save to a local file for later replay
4. Useful for reproducing compiler behavior differences

### List available compilers
1. `GET /api/compilers` to discover our custom-built compilers
2. Verify they're properly registered in CE config
3. Health-check after setup
