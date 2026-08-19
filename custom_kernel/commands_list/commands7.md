| Command          | Purpose / Use Case                                             | Example                   |
| ---------------- | -------------------------------------------------------------- | ------------------------- |
| **accton** | Enables process accounting (tracks commands executed by users) | `sudo accton on`        |
| **bg**     | Sends a suspended job to background                            | `bg %1`                 |
| **chrt**   | Changes process scheduling priority (real-time scheduling)     | `sudo chrt -r 50 PID`   |
| **fg**     | Brings background/suspended job to foreground                  | `fg %1`                 |
| **kill**   | Terminates a process                                           | `kill 1234`             |
| **mpstat** | Shows CPU usage statistics per processor                       | `mpstat`                |
| **pidof**  | Finds Process ID (PID) of a running program                    | `pidof python`          |
| **pmap**   | Shows memory map of a process                                  | `pmap PID`              |
| **ps**     | Lists running processes                                        | `ps aux`                |
| **top**    | Real-time process monitoring                                   | `top`                   |
| **htop**   | Interactive process viewer (better than top)                   | `htop`                  |
| **strace** | Traces system calls made by a process                          | `strace ls`             |
| **time**   | Measures execution time of command                             | `time python script.py` |
| **watch**  | Repeats command output continuously                            | `watch df -h`           |
| **vmstat** | Shows memory, CPU, swap, process statistics                    | `vmstat`                |
| **uptime** | Shows how long system has been running                         | `uptime`                |
| **w**      | Shows logged-in users + what they are doing                    | `w`                     |
