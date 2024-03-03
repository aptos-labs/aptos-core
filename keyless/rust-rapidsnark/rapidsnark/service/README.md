# RapidSnark as a Service

This folder contains the config file to use `proverServer` as a service 
at Linux, with `systemd`.

Just copy the file `rapidsnark.service` to `/etc/systemd/system/rapidsnark.service`
and update the `ExecStart` parameter with the correct path for binary, `.dat` file
and `.zkey` file.

After save the file run:

```
$ sudo systemctl daemon-reload
```

And you can

```
$ sudo service rapidsnark start | stop | reload | status
```

