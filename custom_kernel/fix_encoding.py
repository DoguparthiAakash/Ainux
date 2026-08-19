import sys

with open('src/mm/pmm.rs', 'rb') as f:
    raw = f.read()

# Decode as latin-1 (every byte is valid), then re-encode as UTF-8
# replacing known Windows-1252 smart characters
text = raw.decode('latin-1')
replacements = [
    ('\x97', '--'),   # em-dash
    ('\x96', '-'),    # en-dash
    ('\x93', '"'),    # left double quote
    ('\x94', '"'),    # right double quote
    ('\x91', "'"),    # left single quote
    ('\x92', "'"),    # right single quote
    ('\x85', '...'),  # ellipsis
    ('\x80', ''),     # euro sign (unlikely but safe)
]
for bad, good in replacements:
    text = text.replace(bad, good)

with open('src/mm/pmm.rs', 'w', encoding='utf-8', newline='\n') as f:
    f.write(text)

print('pmm.rs encoding fixed successfully')
