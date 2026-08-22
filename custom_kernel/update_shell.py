import os, glob, re

cmds = set()
for f in glob.glob('commands_list/*.md'):
    with open(f, 'r', encoding='utf-8') as file:
        for line in file:
            if line.startswith('| **'):
                cmd = line.split('|')[1].replace('*', '').replace('`', '').strip()
                if ' (' in cmd:
                    cmd = cmd.split(' (')[0]
                if cmd:
                    cmds.add(cmd)

shell_rs_path = 'src/shell.rs'
with open(shell_rs_path, 'r', encoding='utf-8') as file:
    content = file.read()

# Find existing match cmd block
match_start = content.find('fn execute_command_inner(cmd: &str, args: &[&str], background: bool) {\n    match cmd {')
if match_start == -1:
    print('Could not find match cmd')
    exit(1)

# Find existing builtins
existing_arms = re.findall(r'\s+"([a-zA-Z0-9_-]+)"\s*=>', content[match_start:match_start+5000])
print('Existing branches in match block:', len(existing_arms))

# Which cmds are missing?
missing_cmds = [c for c in cmds if c not in existing_arms]
missing_cmds = [c for c in missing_cmds if re.match(r'^[a-z0-9_-]+$', c)] # valid rust branch strings

missing_cmds.sort()

# Also need to add 'chess'
if 'chess' not in existing_arms:
    missing_cmds.append('chess')

print('Missing commands:', len(missing_cmds))

# We will generate the Rust code for the missing commands
new_arms = []
for c in missing_cmds:
    if c == 'chess':
        new_arms.append(f'            "{c}" => crate::games::chess::ui::run(),')
    else:
        new_arms.append(f'            "{c}" => cmd_stub("{c}"),')

# find the end of the match block (where the _ => ... branch is)
catch_all_idx = content.find('            _ => {', match_start)

new_match_block = '\n'.join(new_arms) + '\n'
new_content = content[:catch_all_idx] + new_match_block + content[catch_all_idx:]

# Now find builtins array:
# let builtins = [ ... ];
builtins_start = new_content.find('let builtins = [')
builtins_end = new_content.find('];', builtins_start)
builtins_str = new_content[builtins_start:builtins_end]

# Extract all built-ins
builtins_list = re.findall(r'"([^"]+)"', builtins_str)

# Add missing to builtins
for c in missing_cmds:
    if not any(b.startswith(c) for b in builtins_list):
        builtins_list.append(c)

builtins_list.sort()

# Reconstruct builtins array with max 10 per line
new_builtins_str = 'let builtins = [\n'
for i in range(0, len(builtins_list), 10):
    chunk = builtins_list[i:i+10]
    line = '        ' + ', '.join([f'"{x}"' for x in chunk]) + ',\n'
    new_builtins_str += line
new_builtins_str += '    '

new_content = new_content[:builtins_start] + new_builtins_str + new_content[builtins_end:]

# Replace another place where builtins are hardcoded (cmd_whereis has a builtins array)
# Let's check if cmd_whereis exists
whereis_idx = new_content.find('let builtins = ["ls", "cp"')
if whereis_idx != -1:
    whereis_end = new_content.find('];', whereis_idx)
    whereis_str = 'let builtins = [\n'
    for i in range(0, len(builtins_list), 10):
        chunk = builtins_list[i:i+10]
        line = '        ' + ', '.join([f'"{x}"' for x in chunk]) + ',\n'
        whereis_str += line
    whereis_str += '    '
    new_content = new_content[:whereis_idx] + whereis_str + new_content[whereis_end:]

with open(shell_rs_path, 'w', encoding='utf-8') as f:
    f.write(new_content)

print('Updated src/shell.rs with', len(missing_cmds), 'new commands.')
