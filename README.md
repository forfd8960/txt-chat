# Txt Chat


## Run

```sh
➜  ~ telnet 0.0.0.0 9090
Trying 0.0.0.0...
Connected to 0.0.0.0.
Escape character is '^]'.
register alice123
register$alice123
727af20893: alice123
```

```sh
2025-10-12T13:23:40.478250Z  INFO txt_chat: server listen on: 0.0.0.0:9090
2025-10-12T13:23:56.932698Z  INFO txt_chat: accept conn from: 127.0.0.1:63448
2025-10-12T13:24:04.202063Z  INFO txt_chat: read message from framed: "register alice123"
2025-10-12T13:24:04.202141Z  WARN txt_chat: error: cmd: register alice123 is not support
2025-10-12T13:24:25.356102Z  INFO txt_chat: read message from framed: "register$alice123"
```