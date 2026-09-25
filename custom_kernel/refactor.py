import os
import glob

gui_dir = 'apps/gui'
files = glob.glob(f'{gui_dir}/*_app.rs')
for filepath in files:
    with open(filepath, 'r') as f:
        content = f.read()
    if 'buffer: buffer.clone(),' in content:
        new_content = content.replace('buffer: buffer.clone(),', 'buffer_ptr: buffer.as_ptr() as u64,')
        with open(filepath, 'w') as f:
            f.write(new_content)
        print(f'Updated {filepath}')
