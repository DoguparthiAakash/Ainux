
| Command           | Purpose / Use Case                                  | Example                        |
| ----------------- | --------------------------------------------------- | ------------------------------ |
| **atd**     | Background daemon that executes scheduled`at`jobs | `sudo systemctl start atd`   |
| **atrm**    | Removes scheduled`at`jobs                         | `atrm 2`                     |
| **atq**     | Lists pending`at`jobs                             | `atq`                        |
| **batch**   | Runs commands when system load becomes low/idle     | `batch`                      |
| **cron**    | Background scheduler service for recurring jobs     | `sudo systemctl status cron` |
| **crontab** | Manage scheduled recurring cron jobs                | `crontab -e`                 |
