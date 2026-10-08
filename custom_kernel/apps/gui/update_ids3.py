import os
import re

directory = r"e:\lh\lsr\Ainux\custom_kernel\apps\gui"
for filename in os.listdir(directory):
    if filename.endswith("_app.rs"):
        path = os.path.join(directory, filename)
        with open(path, "r", encoding="utf-8") as f:
            content = f.read()

        changed = False

        if "WindowClosed => {" not in content:
            # Try to find match something {
            content, count = re.subn(r"match\s+([a-zA-Z_]+)\s*\{\n", r"match \1 {\n                crate::gui::wm::GuiEvent::WindowClosed => { return; },\n                GuiEvent::WindowClosed => { return; },\n", content)
            if count > 0:
                changed = True
            
        if changed:
            with open(path, "w", encoding="utf-8") as f:
                f.write(content)
            print(f"Updated {filename}")

print('Done')
