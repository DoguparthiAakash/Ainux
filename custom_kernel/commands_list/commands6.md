
| Command            | Purpose / Use Case                              | Example                             |
| ------------------ | ----------------------------------------------- | ----------------------------------- |
| **groupadd** | Creates a new group                             | `sudo groupadd developers`        |
| **groupdel** | Deletes an existing group                       | `sudo groupdel developers`        |
| **groupmod** | Modifies group properties                       | `sudo groupmod -n dev developers` |
| **groups**   | Shows groups a user belongs to                  | `groups username`                 |
| **gpasswd**  | Manages group members and passwords             | `sudo gpasswd -a user developers` |
| **grpck**    | Checks integrity of group files                 | `sudo grpck`                      |
| **grpconv**  | Converts group passwords to shadow group format | `sudo grpconv`                    |
