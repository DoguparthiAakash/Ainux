# Subsystem: NuxV Graphics Engine

## 1. Structure
The engine is the visual heart of Ainux, located in `src/drivers/video.rs`:
*   **`Framebuffer` (struct)**: Direct mapping of the physical video memory.
*   **`Point` & `Size` (structs)**: Geometry primitives.
*   **`Color` (type)**: 32-bit ARGB color definitions.

## 2. Functionalities
*   **TUI Rendering**: High-performance text rendering with custom fonts.
*   **Pixel Manipulation**: Direct access to the framebuffer for custom graphics.
*   **Double Buffering**: (Planned) Eliminating flicker during complex redraws.

## 3. Layers
*   **Layer 3 (Shell UI)**: Higher-level windowing and widget logic.
*   **Layer 2 (NuxV Core)**: Primitive drawing (lines, rects, glyphs).
*   **Layer 1 (BGA/VBE)**: Hardware configuration and memory mapping.

## 4. UML (Rendering Pipeline)
```mermaid
graph LR
    Command[Draw Command] --> Raster[Rasterizer]
    Raster --> Buff[Back Buffer]
    Buff --> Blit[Blitter]
    Blit --> FB[Physical Framebuffer]
```

## 5. Usage
### Internal API:
*   `video::put_pixel(x, y, color)`: Draw a single point.
*   `video::put_str(s)`: Render text at the current cursor position.
### Shell Interaction:
*   `wallpaper`: Changes the desktop background (planned).

## 6. Approaches
*   **Linear Framebuffer**: Bypasses legacy VGA modes for modern, high-res 32-bit color.
*   **Bitmap Fonts**: Uses a compiled-in bitmap font for instant text rendering without filesystem dependencies.
*   **Hardware Detection**: Automatically detects VBE/BGA extensions to set the optimal resolution.
