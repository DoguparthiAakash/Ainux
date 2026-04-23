import re
import os

path = r'e:\bsd_parent\Ainux\custom_kernel\src\net\mod.rs'
with open(path, 'rb') as f:
    content = f.read().decode('utf-8')

# Regex for accept
content = re.sub(
    r'if !tcp_socket\.is_open\(\) \{\r?\n\s+tcp_socket\.listen\(22\)\.unwrap\(\);\r?\n\s+\}',
    r'if !tcp_socket.is_open() {\r\n                                    let port = if fd > 100 { 8080 } else { 22 };\r\n                                    tcp_socket.listen(port).unwrap();\r\n                                }',
    content
)

# Regex for bind
content = re.sub(
    r'if let Some\(entry\) = task\.fds\.get_entry\(fd\) \{.*?\r?\n\s+return 0;\r?\n\s+\}',
    r'if let Ok(handle) = task.fds.get_handle(fd) {\r\n                return 0;\r\n            }',
    content,
    flags=re.DOTALL
)

with open(path, 'wb') as f:
    f.write(content.encode('utf-8'))
