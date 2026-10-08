import os
import re

directory = r"e:\lh\lsr\Ainux\custom_kernel\apps\gui"
for filename in os.listdir(directory):
    if filename.endswith("_app.rs"):
        path = os.path.join(directory, filename)
        with open(path, "r", encoding="utf-8") as f:
            content = f.read()

        # Remove the wrong injections
        content = content.replace("                crate::gui::wm::GuiEvent::WindowClosed => { return; },\n", "")
        content = content.replace("                GuiEvent::WindowClosed => { return; },\n", "")

        # Now, correctly inject it right after match event { or match ev { ONLY IF it's inside or event in pop_events or or ev in pop_events
        # Since rust files usually have or ev in ... pop_events then match ev {
        # Let's just look for match ev { or match event {
        
        # Or even simpler, manually check if the match is for the event variable:
        content = re.sub(r"(for\s+(ev|event)\s+in\s+.*?pop_events.*?\n\s*match\s+\2\s*\{\n)", r"\1                crate::gui::wm::GuiEvent::WindowClosed => { return; },\n", content)
            
        with open(path, "w", encoding="utf-8") as f:
            f.write(content)
        print(f"Fixed {filename}")

print('Done')
