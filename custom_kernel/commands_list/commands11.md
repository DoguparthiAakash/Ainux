
| Command                                            | Purpose / Use Case                                  | Example                               |
| -------------------------------------------------- | --------------------------------------------------- | ------------------------------------- |
| **arp**                                      | Shows/manages ARP table (IP ↔ MAC mapping)         | `arp -a`                            |
| **curl**                                     | Transfer data from/to servers (HTTP, APIs)          | `curl https://api.github.com`       |
| **host**                                     | DNS lookup for domain/IP                            | `host google.com`                   |
| **hostid**                                   | Shows system’s unique host ID                      | `hostid`                            |
| **hostname**                                 | Shows/sets system hostname                          | `hostname`                          |
| **hostnamectl** *(image says hostnamecti)* | Manage hostname using systemd                       | `hostnamectl set-hostname mypc`     |
| **ifconfig**                                 | Shows/configures network interfaces (older command) | `ifconfig`                          |
| **iftop**                                    | Real-time network bandwidth monitor                 | `sudo iftop`                        |
| **ip**                                       | Modern network configuration tool                   | `ip addr`                           |
| **ipcrm**                                    | Removes IPC resources (shared memory, semaphores)   | `ipcrm -m 12345`                    |
| **ipcs**                                     | Shows IPC resources                                 | `ipcs`                              |
| **iptables**                                 | Firewall configuration                              | `sudo iptables -L`                  |
| **iptables-save**                            | Saves firewall rules                                | `sudo iptables-save`                |
| **iwconfig**                                 | Configure wireless interfaces                       | `iwconfig`                          |
| **nc (netcat)**                              | TCP/UDP connection testing, port communication      | `nc -zv localhost 80`               |
| **netstat**                                  | Shows network connections, ports, routing           | `netstat -tulnp`                    |
| **nmcli**                                    | Command-line NetworkManager tool                    | `nmcli device status`               |
| **nslookup**                                 | DNS query tool                                      | `nslookup google.com`               |
| **ping**                                     | Tests connectivity to another host                  | `ping 8.8.8.8`                      |
| **rcp**                                      | Remote file copy (older insecure copy tool)         | `rcp file user@host:/tmp`           |
| **route**                                    | Shows/manages routing table                         | `route -n`                          |
| **rsync**                                    | Fast file synchronization                           | `rsync -av src/ dest/`              |
| **scp**                                      | Secure file copy over SSH                           | `scp file.txt user@server:/home`    |
| **ssh**                                      | Secure remote login                                 | `ssh user@192.168.1.10`             |
| **tracepath**                                | Shows network route path                            | `tracepath google.com`              |
| **traceroute**                               | Tracks route packets take                           | `traceroute google.com`             |
| **vnstat**                                   | Network traffic statistics                          | `vnstat`                            |
| **wget**                                     | Downloads files from internet                       | `wget https://example.com/file.zip` |
