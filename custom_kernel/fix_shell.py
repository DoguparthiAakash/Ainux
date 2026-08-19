import re

filepath = 'e:/Ainux/Ainux/custom_kernel/src/shell.rs'
with open(filepath, 'r', encoding='utf-8') as f:
    content = f.read()

# Fix match arguments
content = content.replace('"fdisk" => cmd_fdisk(&args),', '"fdisk" => cmd_fdisk(),')
content = content.replace('"top" => cmd_top(&args),', '"top" => cmd_top(),')

duplicates = ['cmd_chgrp', 'cmd_fdisk', 'cmd_groupadd', 'cmd_hostname', 'cmd_kill', 'cmd_ping', 'cmd_scp', 'cmd_top', 'cmd_useradd', 'cmd_userdel']

for dup in duplicates:
    parts = content.split(f'fn {dup}(')
    if len(parts) > 2:
        last_part = parts[-1]
        end_idx = last_part.find('}')
        if end_idx != -1:
            parts[-1] = last_part[end_idx+1:]
            content = f'fn {dup}('.join(parts[:-1]) + parts[-1]

with open(filepath, 'w', encoding='utf-8') as f:
    f.write(content)

print('Fixed shell.rs')
