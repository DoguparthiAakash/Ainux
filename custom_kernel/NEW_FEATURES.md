# Ainux Industrialization: New Commands & Functions

This document summarizes the new shell commands and kernel functions added during the UI Personalization and System Persistence phase.

## 🐚 Shell Commands (src/shell.rs)

| Command | Purpose | Usage |
| :--- | :--- | :--- |
| `dcustom` | Launches the **Dynamic Personalization Hub**. Allows real-time theme editing (Background, Accent, Selection, etc.). | `dcustom` |
| `save` | Manually persists the current system configuration (including themes and hostname) to `/etc/ainux.conf`. | `save` |
| `hostname` | Gets or sets the system's hostname. Changes are transient until `save` is called. | `hostname [new_name]` |
| `settings` | A unified interface for system-wide configuration, including UI and kernel parameters. | `settings` |
| `tm` | Advanced Task Manager with real-time CPU/Memory tracking and process tree visualization. | `tm` |

---

## 🛠️ Internal Functions & APIs

### 1. Configuration Persistence (`src/config.rs`)
*   **`config::load()`**: Parses `/etc/ainux.conf` and populates the global `CONFIG` object. Automatically restores theme colors and system identity on boot.
*   **`config::save()`**: Serializes the `SystemConfig` struct to disk. Uses an atomic-style overwrite to ensure `/etc/ainux.conf` integrity.
*   **`config::sync_from_theme()`**: Bridges the runtime `THEME` state with the persistent `CONFIG` object.

### 2. UI Rendering & Theme Engine (`src/drivers/video.rs`)
*   **`THEME` Mutex**: A global static containing the active `Theme` struct. Every component in the kernel (Desktop, Window Manager, Apps) reads from here.
*   **`Theme` Struct**:
    *   `bg`: Base background color.
    *   `fg`: Primary text color.
    *   `accent`: Highlights and active borders.
    *   `root`: Desktop/Background wallpaper color.
    *   `dialog_bg`: Background for floating windows and dialogs.
    *   `sel`: Text selection and focus indicator color.
*   **`theme_colors()`**: A helper in `dcustom.rs` that provides a high-level `Palette` for UI elements, including calculated shadows and border colors.

### 3. Character Synthesis (`src/c/text.c`)
*   **`c_draw_char(uint32_t c, ...)`**: Completely refactored to support **32-bit Unicode**.
    *   Supports synthetic glyph generation for box-drawing characters (double lines, corners).
    *   Prevents 8-bit truncation issues that caused `═` to render as `P`.

### 4. Interactive Dialogs (`src/apps/dcustom.rs`)
*   **`draw_dialog(d: &Dialog)`**: Theme-aware window renderer with realistic shadows and accent-colored borders.
*   **`main_personalization()`**: Fluid navigation loop supporting Arrow Keys + Tab for navigating between lists and buttons.

---

## 📂 Key Files Modified
*   `src/apps/dcustom.rs`: The core of the personalization UI.
*   `src/config.rs`: The persistence layer.
*   `src/drivers/video.rs`: The central aesthetic source of truth.
*   `src/c/text.c`: The low-level glyph rendering engine.
*   `build_only.sh`: Updated build pipeline to ensure C/Rust FFI consistency.
