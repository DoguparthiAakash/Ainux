import re

with open("src/shell.rs", "r") as f:
    content = f.read()

# Extract list of commands from execute_command_inner
m = re.search(r"fn execute_command_inner.*?\{(.*?)\n}", content, re.DOTALL)
commands = set()
if m:
    body = m.group(1)
    for line in body.split("\n"):
        match = re.search(r"\"([^\"]+)\"\s*(?:\|\s*\"([^\"]+)\")*\s*=>", line)
        if match:
            commands.add(match.group(1))
            if match.group(2):
                commands.add(match.group(2))

# Pre-defined descriptions and synopses
# format: "cmd": ("description", "synopsis")
cmd_info = {
    "ls": ("list directory contents", "ls [FILE]..."),
    "cd": ("change the shell working directory", "cd [DIR]"),
    "cat": ("concatenate files and print on the standard output", "cat [FILE]..."),
    "ps": ("report a snapshot of the current processes", "ps"),
    "top": ("display tasks and system info", "top"),
    "sysctl": ("configure kernel parameters at runtime", "sysctl [NAME[=VALUE]]"),
    "fdisk": ("manipulate disk partition table", "fdisk"),
    "sensors": ("print sensors information", "sensors"),
    "speakertest": ("test the PC speaker audio", "speakertest"),
    "sudo": ("execute a command as root", "sudo [COMMAND]"),
    "sort": ("sort lines of text files", "sort [FILE]"),
    "whereis": ("locate the binary for a command", "whereis [COMMAND]"),
    "locate": ("find files by name", "locate [NAME]"),
    "help": ("display information about available commands", "help"),
    "clear": ("clear the terminal screen", "clear"),
    "echo": ("display a line of text", "echo [STRING]..."),
    "pwd": ("print name of current/working directory", "pwd"),
    "mkdir": ("make directories", "mkdir [DIRECTORY]"),
    "rm": ("remove files or directories", "rm [FILE]"),
    "cp": ("copy files and directories", "cp [SOURCE] [DEST]"),
    "mv": ("move (rename) files", "mv [SOURCE] [DEST]"),
    "touch": ("change file timestamps (or create empty files)", "touch [FILE]"),
    "grep": ("print lines that match patterns", "grep [PATTERN] [FILE]"),
    "find": ("search for files in a directory hierarchy", "find [PATH] [EXPRESSION]"),
    "history": ("display the command history list", "history"),
    "man": ("an interface to the system reference manuals", "man [COMMAND]"),
    "nano": ("a small and friendly text editor", "nano [FILE]"),
    "vim": ("Vi IMproved, a programmer's text editor", "vim [FILE]"),
    "nvix": ("nvi text editor", "nvix [FILE]"),
    "tree": ("list contents of directories in a tree-like format", "tree [DIRECTORY]"),
    "uname": ("print system information", "uname"),
    "uptime": ("tell how long the system has been running", "uptime"),
    "whoami": ("print effective user ID", "whoami"),
    "date": ("print or set the system date and time", "date"),
    "free": ("display amount of free and used memory in the system", "free"),
    "df": ("report file system disk space usage", "df"),
    "du": ("estimate file space usage", "du [FILE]"),
    "ping": ("send ICMP ECHO_REQUEST to network hosts", "ping [HOST]"),
    "ip": ("show / manipulate routing, network devices, interfaces and tunnels", "ip [OPTIONS]"),
    "netstat": ("print network connections, routing tables, interface statistics", "netstat"),
    "wget": ("the non-interactive network downloader", "wget [URL]"),
    "ssh": ("OpenSSH SSH client (remote login program)", "ssh [USER@]HOST"),
    "chmod": ("change file mode bits", "chmod [MODE] [FILE]"),
    "chown": ("change file owner and group", "chown [OWNER][:[GROUP]] [FILE]"),
    "tar": ("an archiving utility", "tar [OPTIONS] [FILE]"),
    "zip": ("package and compress (archive) files", "zip [OPTIONS] [FILE]"),
    "unzip": ("list, test and extract compressed files in a ZIP archive", "unzip [FILE]"),
    "su": ("run a command with substitute user and group ID", "su [USER]"),
    "reboot": ("reboot the system", "reboot"),
    "shutdown": ("halt, power-off or reboot the machine", "shutdown"),
    "mount": ("mount a filesystem", "mount [DEVICE] [DIR]"),
    "umount": ("unmount file systems", "umount [DIR]"),
    "dmesg": ("print or control the kernel ring buffer", "dmesg"),
    "lspci": ("list all PCI devices", "lspci"),
    "lsusb": ("list USB devices", "lsusb"),
    "lsblk": ("list block devices", "lsblk"),
    "stat": ("display file or file system status", "stat [FILE]"),
    "file": ("determine file type", "file [FILE]"),
    "wc": ("print newline, word, and byte counts for each file", "wc [FILE]"),
    "head": ("output the first part of files", "head [FILE]"),
    "tail": ("output the last part of files", "tail [FILE]"),
    "less": ("opposite of more, display file contents pager", "less [FILE]"),
    "more": ("file perusal filter for crt viewing", "more [FILE]"),
    "watch": ("execute a program periodically, showing output fullscreen", "watch [COMMAND]"),
    "kill": ("send a signal to a process", "kill [PID]"),
    "killall": ("kill processes by name", "killall [NAME]"),
    "pkill": ("look up or signal processes based on name and other attributes", "pkill [NAME]"),
    "bg": ("run jobs in the background", "bg"),
    "fg": ("run jobs in the foreground", "fg"),
    "jobs": ("display status of jobs in the current session", "jobs"),
    "alias": ("define or display aliases", "alias [NAME[=VALUE]]"),
    "unalias": ("remove alias definitions", "unalias [NAME]"),
    "export": ("set the export attribute for variables", "export [NAME[=VALUE]]"),
    "unset": ("unset values and attributes of variables", "unset [NAME]"),
    "env": ("run a program in a modified environment", "env"),
    "source": ("execute commands from a file in the current shell", "source [FILE]"),
    "exit": ("cause the shell to exit", "exit"),
    "passwd": ("change user password", "passwd [USER]"),
    "run": ("run an executable file", "run [FILE]"),
    "nuxc": ("Ainux C compiler", "nuxc [FILE]"),
    "gcc": ("Ainux C compiler", "gcc [FILE]"),
    "clang": ("Ainux C compiler", "clang [FILE]"),
    "nuxa": ("Ainux assembler", "nuxa [FILE]"),
    "nuxv": ("Ainux virtual machine", "nuxv [FILE]"),
    "exec": ("execute a command", "exec [COMMAND]"),
    "view": ("view a file", "view [FILE]"),
    "code": ("open text editor", "code [FILE]"),
    "rmdir": ("remove empty directories", "rmdir [DIRECTORY]"),
    "hfetch": ("display system info", "hfetch"),
    "gputest": ("test the GPU", "gputest"),
    "sudoku": ("play sudoku", "sudoku"),
    "chess": ("play chess", "chess"),
    "chess3d": ("play 3D chess", "chess3d"),
    "2048": ("play 2048", "2048"),
    "minesweeper": ("play minesweeper", "minesweeper"),
    "wifi": ("configure wifi", "wifi [OPTIONS]"),
    "sshd": ("OpenSSH SSH daemon", "sshd"),
    "fetch": ("fetch a file from a URL", "fetch [URL]"),
    "clock": ("display a clock", "clock"),
    "cal": ("display a calendar", "cal"),
    "ln": ("make links between files", "ln [TARGET] [LINK_NAME]"),
    "discover": ("discover devices", "discover"),
    "checkpoint": ("create a system checkpoint", "checkpoint"),
    "restore": ("restore a system checkpoint", "restore"),
    "remorph": ("remorph the system", "remorph"),
    "fuel": ("check system fuel/resources", "fuel"),
    "test_iso": ("test the ISO image", "test_iso"),
    "grant": ("grant privileges", "grant [USER] [PRIVILEGE]"),
    "useradd": ("add a new user", "useradd [USER]"),
    "userdel": ("delete a user", "userdel [USER]"),
    "write": ("write text to a file", "write [FILE] [TEXT]"),
    "save": ("save the current state", "save"),
    "format": ("format a storage device", "format [DEVICE]"),
    "time": ("run programs and summarize system resource usage", "time [COMMAND]"),
    "timezone": ("set or display timezone", "timezone [ZONE]"),
}

man_func_start = """fn cmd_man(args: &[&str]) {
    if args.len() < 2 {
        sh_put_str("What manual page do you want?\\nExample: man ls\\n");
        return;
    }

    match args[1] {
"""

arms = []
for cmd in sorted(commands):
    d, s = cmd_info.get(cmd, (f"execute the {cmd} command", cmd))
    upper_cmd = cmd.upper()
    arms.append(f"""        "{cmd}" => {{
            sh_put_str("{upper_cmd}(1)              Sovereign User Commands             {upper_cmd}(1)\\n\\n");
            sh_put_str("NAME\\n       {cmd} - {d}\\n\\n");
            sh_put_str("SYNOPSIS\\n       {s}\\n\\n");
            sh_put_str("DESCRIPTION\\n       {d}. See Ainux documentation for more info.\\n");
        }},""")

# Also explicitly add man if it's missing from execute_command_inner regex (though it should be there)
if "man" not in commands:
    cmd = "man"
    d, s = cmd_info["man"]
    upper_cmd = cmd.upper()
    arms.append(f"""        "{cmd}" => {{
            sh_put_str("{upper_cmd}(1)              Sovereign User Commands             {upper_cmd}(1)\\n\\n");
            sh_put_str("NAME\\n       {cmd} - {d}\\n\\n");
            sh_put_str("SYNOPSIS\\n       {s}\\n\\n");
            sh_put_str("DESCRIPTION\\n       {d}. See Ainux documentation for more info.\\n");
        }},""")

man_func_end = """        _ => sh_put_str(&alloc::format!("No manual entry for {}\\n", args[1])),
    }
}"""

new_man_func = man_func_start + "\n".join(arms) + "\n" + man_func_end

# Replace the existing cmd_man with new_man_func
match = re.search(r"fn cmd_man\(args: &\[&str\]\) \{.*?\n\}\n", content, re.DOTALL)
if match:
    new_content = content[:match.start()] + new_man_func + "\n" + content[match.end():]
    with open("src/shell.rs", "w") as f:
        f.write(new_content)
    print("Successfully replaced cmd_man in src/shell.rs")
else:
    print("Could not find cmd_man in src/shell.rs")
