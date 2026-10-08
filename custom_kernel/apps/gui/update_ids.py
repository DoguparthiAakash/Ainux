import os
import re

directory = r"e:\lh\lsr\Ainux\custom_kernel\apps\gui"
for filename in os.listdir(directory):
    if filename.endswith("_app.rs"):
        path = os.path.join(directory, filename)
        with open(path, "r", encoding="utf-8") as f:
            content = f.read()

        # Replace hardcoded ID
        content = re.sub(r"let id = \d+;", "let id = crate::gui::wm::generate_window_id();", content)

        # Ensure WindowClosed is handled
        if "WindowClosed => {" not in content:
            # We look for match ev { and insert WindowClosed right after it
            content = re.sub(r"match\s+ev\s*\{\n", "match ev {\n                crate::gui::wm::GuiEvent::WindowClosed => { return; },\n                GuiEvent::WindowClosed => { return; },\n", content)
            
        with open(path, "w", encoding="utf-8") as f:
            f.write(content)

print('Done')
