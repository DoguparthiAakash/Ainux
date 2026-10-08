import os
import re

directory = r"e:\lh\lsr\Ainux\custom_kernel\apps\gui"
for filename in os.listdir(directory):
    if filename.endswith(".rs") and filename != "wm.rs" and filename != "compositor.rs":
        path = os.path.join(directory, filename)
        with open(path, "r", encoding="utf-8") as f:
            content = f.read()

        changed = False

        if re.search(r"pop_events\(\d+\)", content):
            content = re.sub(r"pop_events\(\d+\)", "pop_events(id)", content)
            changed = True
            
        if re.search(r"id:\s*\d+,", content):
            content = re.sub(r"id:\s*\d+,", "id,", content)
            changed = True

        if changed:
            with open(path, "w", encoding="utf-8") as f:
                f.write(content)
            print(f"Updated {filename}")

print('Done')
