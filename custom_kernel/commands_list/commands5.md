| Command            | Purpose / Use Case                                                 | Example                              |
| ------------------ | ------------------------------------------------------------------ | ------------------------------------ |
| **chage**    | Change password aging policy for a user                            | `sudo chage -l user`               |
| **id**       | Shows user ID (UID), group ID (GID), groups                        | `id`                               |
| **userdel**  | Deletes a user account                                             | `sudo userdel user1`               |
| **chfn**     | Changes user information (full name, phone, etc.)                  | `chfn`                             |
| **passwd**   | Changes user password                                              | `passwd`                           |
| **usermod**  | Modifies an existing user account                                  | `sudo usermod -aG sudo user1`      |
| **chsh**     | Changes default login shell                                        | `chsh -s /bin/bash`                |
| **pinky**    | Displays brief user information                                    | `pinky username`                   |
| **users**    | Shows currently logged-in users                                    | `users`                            |
| **chpasswd** | Changes passwords in batch/script mode                             | `echo "user:pass" \| sudo chpasswd` |
| **username** | Displays current username (less common command; usually`whoami`) | `username`                         |
| **who**      | Shows users currently logged into system                           | `who`                              |
| **finger**   | Shows detailed user information                                    | `finger user1`                     |
| **useradd**  | Creates a new user account                                         | `sudo useradd john`                |
| **whoami**   | Shows current logged-in username                                   | `whoami`                           |
