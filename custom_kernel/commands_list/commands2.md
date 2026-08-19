
| Command            | Purpose / Use Case                          | Example                                        |
| ------------------ | ------------------------------------------- | ---------------------------------------------- |
| **basename** | Extracts filename from a path               | `basename /home/user/file.txt`→`file.txt` |
| **cp**       | Copies files/directories                    | `cp a.txt b.txt`                             |
| **echo**     | Prints text/variables                       | `echo "Hello"`                               |
| **less**     | View file content page by page (scrollable) | `less file.txt`                              |
| **od**       | Displays file in octal/hex/binary format    | `od file.txt`                                |
| **shred**    | Securely deletes file by overwriting data   | `shred secret.txt`                           |
| **tee**      | Saves output to file while displaying it    | `ls                                            |
| **cat**      | Displays file contents                      | `cat file.txt`                               |
| **cpio**     | Copies files to/from archive                | `find .                                        |
| **expand**   | Converts tabs into spaces                   | `expand file.txt`                            |
| **ln**       | Creates hard/symbolic links                 | `ln -s file link`                            |
| **paste**    | Merges lines from files side by side        | `paste file1 file2`                          |
| **sort**     | Sorts file contents                         | `sort names.txt`                             |
| **touch**    | Creates empty file/updates timestamp        | `touch test.txt`                             |
| **cksum**    | Calculates checksum for file integrity      | `cksum file.txt`                             |
| **csplit**   | Splits file based on patterns               | `csplit file.txt '/Chapter/'`                |
| **file**     | Identifies file type                        | `file image.png`                             |
| **locate**   | Finds files quickly                         | `locate python`                              |
| **readlink** | Shows target of symbolic link               | `readlink shortcut`                          |
| **split**    | Splits large file into smaller files        | `split -l 100 big.txt`                       |
| **unexpand** | Converts spaces into tabs                   | `unexpand file.txt`                          |
| **cmp**      | Compares two files byte by byte             | `cmp file1 file2`                            |
| **cut**      | Extracts specific columns/fields            | `cut -d',' -f1 file.csv`                     |
| **fold**     | Wraps long lines to fixed width             | `fold -w 20 file.txt`                        |
| **look**     | Searches words in sorted dictionary/file    | `look app`                                   |
| **rename**   | Renames files in bulk                       | `rename 's/.txt/.md/' *.txt`                 |
| **tac**      | Displays file in reverse line order         | `tac file.txt`                               |
| **uniq**     | Removes duplicate adjacent lines            | `uniq file.txt`                              |
| **compress** | Compresses files (older Unix utility)       | `compress file.txt`                          |
| **diff**     | Shows differences between files             | `diff a.txt b.txt`                           |
| **head**     | Shows first lines of file                   | `head -10 file.txt`                          |
| **more**     | View file page by page                      | `more file.txt`                              |
| **rev**      | Reverses characters in each line            | `echo hello                                    |
| **tail**     | Shows last lines of file                    | `tail -10 log.txt`                           |
| **wc**       | Counts words/lines/characters               | `wc file.txt`                                |
| **diff3**    | Compares 3 files                            | `diff3 file1 file2 file3`                    |
| **join**     | Joins files based on matching field         | `join file1 file2`                           |
| **mv**       | Moves/renames files                         | `mv old.txt new.txt`                         |
| **rm**       | Deletes files                               | `rm file.txt`                                |
| **tar**      | Archives multiple files                     | `tar -cvf backup.tar folder/`                |
