import re

with open("src/shell.rs", "r") as f:
    content = f.read()

m = re.search(r"fn execute_command_inner.*?\{(.*?)\n}", content, re.DOTALL)
if not m:
    print("execute_command_inner not found")
else:
    body = m.group(1)
    commands = set()
    for line in body.split("\n"):
        match = re.search(r"\"([^\"]+)\"\s*(?:\|\s*\"([^\"]+)\")*\s*=>", line)
        if match:
            commands.add(match.group(1))
            if match.group(2):
                commands.add(match.group(2))
    
    print("Found commands:")
    print(", ".join(sorted(commands)))
