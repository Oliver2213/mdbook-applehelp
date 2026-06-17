Now I have all the information needed. Let me write the spec document.

```markdown
# mdbook-applehelp — Specification

> An mdBook backend that generates a macOS Apple Help Book (`.help` bundle) from mdBook source, with optional search index generation via `hiutil`.

## 1. Overview

`mdbook-applehelp` is a standard mdBook backend executable. When mdBook invokes it, it receives the book's `RenderContext` as JSON on stdin and produces a `.help` bundle suitable for inclusion in a macOS app bundle. It uses chapter data from the JSON directly—no HTML re-parsing.

## 2. mdBook Backend Protocol

mdBook backends follow a simple protocol <kcite></kcite>:

- An executable named `mdbook-foo` is invoked when `[output.foo]` exists in `book.toml`.
- mdBook sends a `RenderContext` as JSON on stdin.
- The backend reads it, generates output in `render_context.destination`, and exits 0 on success.

The `RenderContext` JSON has this structure <kcite></kcite>:

```json
{
  "version": "0.4.x",
  "root": "/path/to/book/root",
  "book": {
    "sections": [
      {
        "Chapter": {
          "name": "Chapter Title",
          "content": "# Markdown content here...",
          "number": [1],
          "sub_items": [],
          "path": "chapter-1.md",
          "source_path": "chapter-1.md",
          "parent_names": []
        }
      },
      { "Separator": null },
      {
        "Chapter": { ... }
      }
    ]
  },
  "config": {
    "book": {
      "title": "My Book",
      "authors": ["Author"],
      "description": "A book",
      "language": "en",
      "src": "src"
    },
    "build": {
      "build_dir": "book"
    }
  },
  "destination": "/path/to/book/root/book/applehelp"
}
```

Each `BookItem::Chapter` in the `sections` array contains a `content` field with **raw Markdown** (already preprocessed). This is the authoritative source—use it directly instead of re-parsing generated HTML <kcite></kcite>.

The `command` field in config lets you pass CLI args to the backend <kcite></kcite>:

```toml
[output.applehelp]
command = "mdbook-applehelp --no-index"
```

**CLI arguments the backend should accept:**

| Flag | Purpose |
|---|---|
| `--no-index` | Skip `hiutil` index generation (overrides config) |
| `--force-index` | Force index generation (overrides config) |
| `--help` | Show usage |
| `supports <renderer>` | mdBook backend supports-check protocol (exit 0 if renderer is "applehelp") |

## 3. Configuration

All configuration lives in `book.toml` under `[output.applehelp]`. Values are inferred from existing mdBook config where possible—each piece of information has one canonical home.

```toml
[output.applehelp]
# Required
help-book-name = "com.example.myapp.help"    # CFBundleHelpBookName (used as bundle ID)
help-book-folder = "MyAppHelp"               # CFBundleHelpBookFolder (folder/bundle name)

# Optional — inferred from [book] if omitted
# title          → defaults to book.title
# description    → defaults to book.description
# language       → defaults to book.language ("en")
# authors        → defaults to book.authors (used in meta)

# Optional — index generation
generate-index = true          # default: true on macOS, ignored on non-macOS
index-format = "both"          # "corespotlight" | "lsm" | "both" (default: "both")

# Optional — styling
theme = "default"              # "default" | path to custom CSS directory
landing-page = "index.html"    # filename for root chapter (default: first chapter)

# Optional — Apple Help meta
icon-file = ""                 # HPDBookIconPath (relative path to icon in bundle)
external-url = ""              # HPDBookRemoteURL (for remote index updates)
```

### Inference Rules (single source of truth)

| `[output.applehelp]` key | Falls back to | Used for |
|---|---|---|
| *(not set)* | `book.title` | Help book title page, `<title>` tags |
| *(not set)* | `book.description` | Meta description |
| *(not set)* | `book.language` | `.lproj` folder name, `lang` attr |
| *(not set)* | `book.authors` | Author meta tags |
| `help-book-name` | **no fallback — required** | `CFBundleHelpBookName`, bundle identifier |
| `help-book-folder` | **no fallback — required** | `CFBundleHelpBookFolder`, bundle directory name |

## 4. Output: Apple Help Book Bundle Structure

The generated `.help` bundle follows Apple's documented structure <kcite></kcite>:

```
<MyAppHelp>.help/
├── Contents/
│   ├── Info.plist                          # Help book plist (§5)
│   └── Resources/
│       ├── en.lproj/                       # One per language
│       │   ├── index.html                  # Landing page (root chapter)
│       │   ├── chapter-1.html
│       │   ├── chapter-2.html
│       │   ├── chapter-1/
│       │   │   └── sub-chapter.html
│       │   ├── <book-title>.cshelpindex    # Core Spotlight index (macOS 10.14+)
│       │   └── <book-title>.helpindex      # LSM index (legacy)
│       ├── en.lproj/
│       │   └── ...                         # Additional languages
│       └── Shared/
│           └── style.css                   # Shared CSS (if theme provides)
```

### Path Mapping: mdBook → Help Book

mdBook chapter `path` values (e.g. `chapter-1/sub-topic.md`) map to HTML files inside the `.lproj/` directory with `.html` extension. The root/landing page gets the filename `index.html`.

Nested mdBook chapters produce nested directories in the bundle, mirroring the source structure.

## 5. Help Book Info.plist

The `Info.plist` inside `Contents/` of the `.help` bundle must contain these keys <kcite></kcite><kcite></kcite>:

```xml
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN"
  "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <!-- Required: bundle identifiers -->
    <key>CFBundleIdentifier</key>
    <string>${help-book-name}</string>

    <key>CFBundleName</key>
    <string>${book.title}</string>

    <key>CFBundleVersion</key>
    <string>1</string>

    <key>CFBundleInfoDictionaryVersion</key>
    <string>6.0</string>

    <!-- Required: help book registration keys -->
    <key>CFBundleHelpBookFolder</key>
    <string>${help-book-folder}</string>

    <key>CFBundleHelpBookName</key>
    <string>${help-book-name}</string>

    <!-- Required for Spotlight search (macOS 10.14+) -->
    <key>HPDBookCSIndexPath</key>
    <string>${book-title}.cshelpindex</string>

    <!-- Required for legacy LSM search -->
    <key>HPDBookLSIndexPath</key>
    <string>${book-title}.helpindex</string>

    <!-- Optional: icon shown in Help Viewer -->
    <key>HPDBookIconPath</key>
    <string></string>              <!-- from config: icon-file -->

    <!-- Optional: remote URL for index updates -->
    <key>HPDBookRemoteURL</key>
    <string></string>             <!-- from config: external-url -->

    <!-- Optional: keyword for Help Viewer access -->
    <key>HPDBookAccessKey</key>
    <string></string>

    <key>HPDBookIndexPathType</key>
    <string>0</string>            <!-- 0 = relative path -->
</dict>
</plist>
```

**Key references:**

| Key | Required | Source |
|---|---|---|
| `CFBundleIdentifier` | Yes | `help-book-name` config |
| `CFBundleHelpBookName` | Yes | `help-book-name` config <kcite></kcite> |
| `CFBundleHelpBookFolder` | Yes | `help-book-folder` config <kcite></kcite> |
| `HPDBookCSIndexPath` | Yes (10.14+) | Derived from book title + `.cshelpindex` <kcite></kcite> |
| `HPDBookLSIndexPath` | Yes (legacy) | Derived from book title + `.helpindex` <kcite></kcite> |
| `HPDBookIconPath` | No | `icon-file` config |
| `HPDBookRemoteURL` | No | `external-url` config |

## 6. HTML Generation from Markdown

### From JSON, Not HTML

The backend receives preprocessed Markdown in each chapter's `content` field. It should:

1. Deserialize the `RenderContext` JSON from stdin.
2. Walk `book.sections` recursively.
3. For each `BookItem::Chapter`, convert `content` (Markdown) to HTML.
4. Wrap in an Apple Help-compatible HTML template.

### Required HTML Meta Tags

Apple Help requires a `<meta>` tag with `AppleTitle` on the landing page <kcite></kcite>:

```html
<!DOCTYPE html>
<html lang="en">
<head>
    <meta http-equiv="Content-Type" content="text/html; charset=utf-8">
    <meta name="AppleTitle" content="My App Help">
    <meta name="AppleIcon" content="Shared/icon.png">
    <title>Chapter Title</title>
    <link rel="stylesheet" href="../Shared/style.css">
</head>
<body>
    <a name="<!-- chapter-path as anchor -->"></a>
    <!-- converted HTML content -->
</body>
</html>
```

- `AppleTitle` meta: set to `book.title` on the landing page; set to chapter name on other pages.
- Named anchors (`<a name="...">`) are derived from `chapter.path` for use with `NSHelpManager.openHelpAnchor(_:inBook:)`.

### Markdown → HTML Conversion

Use **`pulldown-cmark`** (the same crate mdBook uses internally) for Markdown-to-HTML conversion. This ensures maximum compatibility with mdBook's Markdown dialect.

## 7. Search Index Generation

### hiutil Commands

macOS provides `hiutil` for generating help search indexes <kcite></kcite>. Two index formats are required for full compatibility <kcite></kcite>:

```bash
# Core Spotlight index (required macOS 10.14+)
hiutil -I corespotlight -Cf "<book-title>.cshelpindex" -a -s en -l en <lproj-dir>

# LSM index (legacy, for older macOS)
hiutil -I lsm -Cf "<book-title>.helpindex" -a -s en -l en <lproj-dir>
```

Flags:
- `-C` — create mode
- `-f <path>` — output file path
- `-I <format>` — index format (`corespotlight` or `lsm`)
- `-a` — index all files
- `-s <lang>` — language code
- `-l <lang>` — language code (label)

### Non-macOS Behavior

`hiutil` is macOS-only. When running on a non-macOS platform with index generation requested:

1. **Emit a warning to stderr.**
2. **Fail the build** (non-zero exit).
3. The warning must include:

```
⚠  BUILD FAILED: Search index generation requires macOS (hiutil is unavailable).

   Apple Help search indexes cannot be generated on this platform because
   `hiutil` is a macOS-exclusive tool. The .help bundle was generated
   without search indexes — Help Viewer search will not work.

   To resolve, do ONE of the following:

   a) Disable index generation in book.toml:
        [output.applehelp]
        generate-index = false

   b) Pass --no-index to skip for this build:
        mdbook build  # then run on macOS:
        mdbook-applehelp --force-index  # (outside mdbook, standalone)

   c) Generate indexes on macOS after building:
        cd <build-dir>/applehelp/<HelpBundle>.help/Contents/Resources/en.lproj
        hiutil -I corespotlight -Cf "<title>.cshelpindex" -a -s en -l en .
        hiutil -I lsm -Cf "<title>.helpindex" -a -s en -l en .
```

4. If `generate-index = false` or `--no-index` is set, the build succeeds on any platform (indexes are simply omitted).

## 8. Xcode Integration Guide (SwiftUI App)

### App Bundle Placement

The `.help` bundle must reside at:

```
<AppBundle>.app/
└── Contents/
    └── Resources/
        └── <MyAppHelp>.help/
```

### App's Info.plist Keys

Add to the **app's** `Info.plist` (not the help book's):

```xml
<key>CFBundleHelpBookFolder</key>
<string>MyAppHelp</string>
<key>CFBundleHelpBookName</key>
<string>com.example.myapp.help</string>
```

### Run-Script Build Phase

Add a Run Script phase in Xcode that builds the mdBook and copies the `.help` bundle into the app. Use Xcode build setting environment variables <kcite></kcite><kcite></kcite>:

```bash
set -euo pipefail

# Xcode-provided environment variables (no need to hardcode these):
#   SRCROOT            — project source root
#   BUILT_PRODUCTS_DIR — built products directory
#   PRODUCT_NAME       — product name
#   PRODUCT_BUNDLE_IDENTIFIER — e.g. com.example.myapp
#   UNLOCALIZED_RESOURCES_FOLDER_PATH — Resources/ path in bundle
#   INFOPLIST_FILE     — path to the app's Info.plist
#   DEVELOPMENT_LANGUAGE — default language (e.g. "en")
#   CURRENT_PROJECT_VERSION — build version
#   MARKETING_VERSION  — marketing version

HELP_SRC="${SRCROOT}/docs"              # mdBook source location
HELP_BUILD_DIR=$(mktemp -d)

# Build the mdBook with applehelp backend
cd "${HELP_SRC}"
mdbook build -d "${HELP_BUILD_DIR}"

# Derive help book folder name from book.toml (avoid hardcoding)
HELP_BUNDLE=$(find "${HELP_BUILD_DIR}/applehelp" -name "*.help" -maxdepth 1 | head -1)

if [ -z "${HELP_BUNDLE}" ]; then
    echo "error: No .help bundle generated by mdbook-applehelp"
    exit 1
fi

# Copy into app bundle's Resources
DEST="${BUILT_PRODUCTS_DIR}/${UNLOCALIZED_RESOURCES_FOLDER_PATH}"
cp -R "${HELP_BUNDLE}" "${DEST}/"

echo "Copied $(basename "${HELP_BUNDLE}") to ${DEST}"

# Verify the app's Info.plist has the help keys (informational)
HELP_FOLDER=$(/usr/libexec/PlistBuddy -c "Print :CFBundleHelpBookFolder" "${INFOPLIST_FILE}" 2>/dev/null || echo "NOT SET")
HELP_NAME=$(/usr/libexec/PlistBuddy -c "Print :CFBundleHelpBookName" "${INFOPLIST_FILE}" 2>/dev/null || echo "NOT SET")

if [ "${HELP_FOLDER}" = "NOT SET" ] || [ "${HELP_NAME}" = "NOT SET" ]; then
    echo "warning: CFBundleHelpBookFolder and/or CFBundleHelpBookName not set in ${INFOPLIST_FILE}"
    echo "  Add these keys to your app's Info.plist:"
    echo "    CFBundleHelpBookFolder = <help-book-folder from book.toml>"
    echo "    CFBundleHelpBookName = <help-book-name from book.toml>"
fi
```

### Xcode Environment Variables Available

| Variable | Description | Usage in script |
|---|---|---|
| `SRCROOT` | Project source root | Locate mdBook source (`docs/`) |
| `BUILT_PRODUCTS_DIR` | Built products directory | Copy `.help` to app bundle |
| `UNLOCALIZED_RESOURCES_FOLDER_PATH` | `Contents/Resources/` path | Destination for `.help` bundle |
| `PRODUCT_BUNDLE_IDENTIFIER` | App bundle ID (e.g. `com.example.myapp`) | Derive `help-book-name` suggestion |
| `PRODUCT_NAME` | App product name | Derive `help-book-folder` suggestion |
| `INFOPLIST_FILE` | Path to app's Info.plist | Verify help keys are set |
| `DEVELOPMENT_LANGUAGE` | Default language | Match `.lproj` folder |
| `MARKETING_VERSION` | Version (e.g. `1.0`) | Could stamp into help bundle |
| `CURRENT_PROJECT_VERSION` | Build number | Could stamp into help bundle |

### Opening Help from SwiftUI

```swift
import AppKit

// Open the entire help book (landing page)
@objc func openHelp() {
    NSApp.showHelp(nil)
}

// Open a specific anchor
func openHelpAnchor(_ anchor: String) {
    NSHelpManager.shared.openHelpAnchor(anchor, inBook: "com.example.myapp.help")
}
```

In SwiftUI, use `.command` modifier or `NSApplication.shared.showHelp(nil)`.

### Help Viewer Cache Clearing (Debug Tip)

Help Viewer aggressively caches. During development, clear the cache:

```bash
rm -rf ~/Library/Caches/com.apple.helpd
rm -rf ~/Library/Preferences/com.apple.helpd.plist
killall helpd
```

## 9. Rust Crate Dependencies

| Crate | Purpose | Notes |
|---|---|---|
| `mdbook` | `RenderContext` deserialization, `Book`/`Chapter` types | Reuse mdBook's own types for perfect JSON compat <kcite></kcite> |
| `serde` + `serde_json` | JSON deserialization from stdin | Required by mdBook backend protocol |
| `pulldown-cmark` | Markdown → HTML | Same engine mdBook uses; ensures dialect parity |
| `plist` | Generate `Info.plist` (XML plist format) | Pure Rust, cross-platform plist writing |
| `clap` | CLI argument parsing | `--no-index`, `--force-index`, `supports` subcommand |
| `env_logger` | Logging / warnings | Structured output to stderr |

### Why `plist` crate?

The `plist` crate (<https://crates.io/crates/plist>) is a pure-Rust, cross-platform library for reading and writing Apple property list files in both XML and binary formats. It does not require macOS—essential for generating `Info.plist` on any platform.

### Why `pulldown-cmark`?

Using the same Markdown parser as mdBook ensures that the HTML output matches what the HTML backend would produce, avoiding rendering discrepancies.

## 10. Build Flow

```
mdbook build
  │
  ├─ Preprocessors run (links, etc.)
  │
  └─ [output.applehelp] backend invoked
       │
       ├─ Read RenderContext JSON from stdin
       │
       ├─ Parse config from context.config
       │    ├─ help-book-name (required)
       │    ├─ help-book-folder (required)
       │    └─ Fallbacks: title, language, description from book config
       │
       ├─ Create .help/Contents/ directory structure
       │
       ├─ Generate Info.plist (§5)
       │
       ├─ Walk book.sections recursively
       │    ├─ For each Chapter:
       │    │    ├─ Convert content (Markdown) → HTML via pulldown-cmark
       │    │    ├─ Wrap in Apple Help HTML template (§6)
       │    │    ├─ Write to <lproj>/<path>.html
       │    │    └─ Record anchor name from chapter.path
       │    └─ Skip Separators
       │
       ├─ Write CSS to Shared/ (from theme)
       │
       └─ Index generation (if enabled & macOS)
            ├─ Run: hiutil -I corespotlight ... → .cshelpindex
            ├─ Run: hiutil -I lsm ...           → .helpindex
            └─ On non-macOS: emit warning + fail (§7)
```

## 11. Error Handling

| Condition | Behavior |
|---|---|
| `help-book-name` not set | Error: "help-book-name is required in [output.applehelp]" |
| `help-book-folder` not set | Error: "help-book-folder is required in [output.applehelp]" |
| `hiutil` unavailable (non-macOS) + index requested | Warning with instructions + exit 1 (§7) |
| `hiutil` unavailable (non-macOS) + `generate-index = false` | Build succeeds, no indexes generated |
| `--no-index` passed | Skip index generation regardless of config |
| Invalid book structure | Error with chapter path context |

## 12. Future Considerations

- **Localization**: `book.language` maps to `.lproj` folders. Multiple languages could be supported via mdBook's `src` directory structure or a future `[output.applehelp.languages]` config.
- **Remote index updates**: `HPDBookRemoteURL` is already configurable for Help Viewer to fetch updated indexes.
- **Anchor generation**: Automatically generate `<a name="...">` from chapter paths for `NSHelpManager.openHelpAnchor` integration.
- **mdBook link rewriting**: Internal mdBook links (`../chapter.md` style) must be rewritten to `.html` paths within the bundle.
```

That's the full spec. Here's a quick summary of the key design decisions:

- **Single source of truth**: `book.title`, `book.language`, `book.description`, and `book.authors` from the existing mdBook config are reused—no duplication.
- **JSON-native**: Walks the `RenderContext` JSON's chapter tree directly; `content` fields contain preprocessed Markdown that's converted with `pulldown-cmark`—no HTML re-parsing.
- **Cross-platform build**: `plist` crate generates the Info.plist on any OS. Only `hiutil` is macOS-only, and the build fails cleanly with actionable instructions when it's unavailable.
- **CLI escape hatches**: `--no-index` and `--force-index` let users override config from the command line via the `command` field in `book.toml`.
- **Xcode integration**: The build script uses `SRCROOT`, `BUILT_PRODUCTS_DIR`, `UNLOCALIZED_RESOURCES_FOLDER_PATH`, `INFOPLIST_FILE`, and `PRODUCT_BUNDLE_IDENTIFIER` to avoid hardcoding anything the build system already knows.