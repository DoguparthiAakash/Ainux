import os

file_path = "e:\\lh\\lsr\\Ainux\\custom_kernel\\core\\shell.rs"
with open(file_path, "r", encoding="utf-8") as f:
    content = f.read()

# Replace fully qualified ones first to avoid partial replacement issues
content = content.replace("crate::drivers::video::put_str_colored", "sh_put_str_colored")
content = content.replace("crate::drivers::video::put_str", "sh_put_str")
content = content.replace("crate::drivers::video::put_char", "sh_put_char")
content = content.replace("crate::drivers::video::clear", "sh_clear")
content = content.replace("crate::drivers::video::flush_screen", "// flush_screen")

# Replace standard ones
content = content.replace("video::put_str_colored", "sh_put_str_colored")
content = content.replace("video::put_str", "sh_put_str")
content = content.replace("video::put_char", "sh_put_char")
content = content.replace("video::clear", "sh_clear")

with open(file_path, "w", encoding="utf-8") as f:
    f.write(content)

print("Replacements complete!")
