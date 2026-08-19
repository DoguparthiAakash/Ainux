import os, glob

missing_cmds = set()
for f in sorted(glob.glob('commands_list/*.md')):
    with open(f, 'r', encoding='utf-8') as file:
        for line in file:
            if line.startswith('| **'):
                cmd = line.split('|')[1].replace('*', '').replace('`', '').strip()
                if ' (' in cmd:
                    cmd = cmd.split(' (')[0]
                if cmd:
                    missing_cmds.add(cmd)
print('Total unique commands:', len(missing_cmds))
print(', '.join(sorted(missing_cmds)))
